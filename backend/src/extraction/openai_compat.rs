//! OpenAI-compatible `/chat/completions` with an image: Ollama, LM Studio, vLLM, OpenRouter...

use async_trait::async_trait;
use base64::Engine;
use serde_json::{Value, json};

use super::{ExtractError, ProviderOutput, ReceiptExtractor, parse_model_json, prompt, schema};
use crate::config::{JsonMode, OpenAiConfig};

pub struct OpenAiCompatExtractor {
    http: reqwest::Client,
    cfg: OpenAiConfig,
}

impl OpenAiCompatExtractor {
    pub fn new(http: reqwest::Client, cfg: OpenAiConfig) -> Self {
        Self { http, cfg }
    }

    fn request_body(&self, image: &[u8], mime: &str) -> Value {
        let data_url = format!(
            "data:{mime};base64,{}",
            base64::engine::general_purpose::STANDARD.encode(image)
        );
        let response_format = match self.cfg.json_mode {
            JsonMode::JsonObject => json!({ "type": "json_object" }),
            JsonMode::JsonSchema => json!({
                "type": "json_schema",
                "json_schema": { "name": "receipt", "schema": schema::json_schema() }
            }),
        };
        // In json_object mode the model only knows the shape from the prompt.
        let system = format!(
            "{}\n\nJSON schema:\n{}",
            prompt::system_prompt(),
            schema::json_schema()
        );
        json!({
            "model": self.cfg.model,
            "temperature": 0,
            "response_format": response_format,
            "messages": [
                { "role": "system", "content": system },
                { "role": "user", "content": [
                    { "type": "text", "text": prompt::user_prompt() },
                    { "type": "image_url", "image_url": { "url": data_url } }
                ] }
            ]
        })
    }
}

#[async_trait]
impl ReceiptExtractor for OpenAiCompatExtractor {
    fn provider(&self) -> &'static str {
        "openai_compat"
    }

    fn model(&self) -> &str {
        &self.cfg.model
    }

    async fn extract(&self, image: &[u8], mime: &str) -> Result<ProviderOutput, ExtractError> {
        let mut req = self
            .http
            .post(format!("{}/chat/completions", self.cfg.base_url))
            .json(&self.request_body(image, mime));
        if let Some(key) = &self.cfg.api_key {
            req = req.bearer_auth(key);
        }
        let resp = req.send().await.map_err(ExtractError::from_reqwest)?;
        let status = resp.status();
        let body = resp.text().await.map_err(ExtractError::from_reqwest)?;
        if !status.is_success() {
            return Err(ExtractError::from_status(status, &body));
        }
        let v: Value = serde_json::from_str(&body)
            .map_err(|e| ExtractError::Transient(format!("invalid response: {e}")))?;
        let choice = &v["choices"][0];
        if choice["finish_reason"].as_str() == Some("length") {
            return Err(ExtractError::Permanent(
                "model hit max tokens; JSON is truncated".into(),
            ));
        }
        let text = choice["message"]["content"].as_str().unwrap_or_default();
        Ok(ProviderOutput {
            extraction: parse_model_json(text)?,
            input_tokens: v["usage"]["prompt_tokens"].as_i64().map(|n| n as i32),
            output_tokens: v["usage"]["completion_tokens"].as_i64().map(|n| n as i32),
        })
    }
}
