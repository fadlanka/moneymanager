//! Google Gemini `generateContent` with structured output (`responseSchema`).

use async_trait::async_trait;
use base64::Engine;
use serde_json::{Value, json};

use super::{ExtractError, ProviderOutput, ReceiptExtractor, parse_model_json, prompt, schema};
use crate::config::GeminiConfig;

const BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";

pub struct GeminiExtractor {
    http: reqwest::Client,
    cfg: GeminiConfig,
}

impl GeminiExtractor {
    pub fn new(http: reqwest::Client, cfg: GeminiConfig) -> Self {
        Self { http, cfg }
    }

    fn request_body(&self, image: &[u8], mime: &str) -> Value {
        let mut generation = json!({
            "temperature": 0,
            "responseMimeType": "application/json",
            "responseSchema": schema::gemini_schema(),
        });
        // Receipt reading needs no long reasoning, and thinking tokens can eat the
        // output budget and truncate the JSON.
        if let Some(budget) = self.cfg.thinking_budget {
            generation["thinkingConfig"] = json!({ "thinkingBudget": budget });
        }
        json!({
            "systemInstruction": { "parts": [{ "text": prompt::system_prompt() }] },
            "contents": [{
                "role": "user",
                "parts": [
                    { "text": prompt::user_prompt() },
                    { "inline_data": {
                        "mime_type": mime,
                        "data": base64::engine::general_purpose::STANDARD.encode(image)
                    } }
                ]
            }],
            "generationConfig": generation
        })
    }
}

#[async_trait]
impl ReceiptExtractor for GeminiExtractor {
    fn provider(&self) -> &'static str {
        "gemini"
    }

    fn model(&self) -> &str {
        &self.cfg.model
    }

    async fn extract(&self, image: &[u8], mime: &str) -> Result<ProviderOutput, ExtractError> {
        let url = format!("{BASE_URL}/models/{}:generateContent", self.cfg.model);
        let resp = self
            .http
            .post(url)
            .header("x-goog-api-key", &self.cfg.api_key)
            .json(&self.request_body(image, mime))
            .send()
            .await
            .map_err(ExtractError::from_reqwest)?;

        let status = resp.status();
        let body = resp.text().await.map_err(ExtractError::from_reqwest)?;
        if !status.is_success() {
            return Err(ExtractError::from_status(status, &body));
        }
        let v: Value = serde_json::from_str(&body)
            .map_err(|e| ExtractError::Transient(format!("invalid Gemini response: {e}")))?;

        let candidate = &v["candidates"][0];
        if let Some(reason) = candidate["finishReason"].as_str() {
            if reason == "MAX_TOKENS" {
                return Err(ExtractError::Permanent(
                    "Gemini stopped at MAX_TOKENS; the JSON is truncated".into(),
                ));
            }
            if reason == "SAFETY" || reason == "RECITATION" {
                return Err(ExtractError::Permanent(format!(
                    "Gemini blocked the reply: {reason}"
                )));
            }
        }
        let text: String = candidate["content"]["parts"]
            .as_array()
            .map(|parts| {
                parts
                    .iter()
                    // Skip thought summaries if a thinking model returns them.
                    .filter(|p| !p["thought"].as_bool().unwrap_or(false))
                    .filter_map(|p| p["text"].as_str())
                    .collect()
            })
            .unwrap_or_default();
        if text.is_empty() {
            return Err(ExtractError::Permanent(format!(
                "Gemini returned no text: {}",
                body.chars().take(300).collect::<String>()
            )));
        }

        let usage = &v["usageMetadata"];
        Ok(ProviderOutput {
            extraction: parse_model_json(&text)?,
            input_tokens: usage["promptTokenCount"].as_i64().map(|n| n as i32),
            output_tokens: usage["candidatesTokenCount"].as_i64().map(|n| n as i32),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn body_has_schema_and_thinking_off() {
        let ex = GeminiExtractor::new(
            reqwest::Client::new(),
            GeminiConfig {
                api_key: "k".into(),
                model: "gemini-2.5-flash".into(),
                thinking_budget: Some(0),
            },
        );
        let body = ex.request_body(b"img", "image/jpeg");
        assert_eq!(
            body["generationConfig"]["thinkingConfig"]["thinkingBudget"],
            0
        );
        assert_eq!(body["generationConfig"]["responseSchema"]["type"], "OBJECT");
        assert_eq!(
            body["contents"][0]["parts"][1]["inline_data"]["data"],
            "aW1n"
        );
    }
}
