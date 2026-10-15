//! Receipt extraction: provider-agnostic interface, providers, and validation.

pub mod gemini;
pub mod mock;
pub mod normalize;
pub mod openai_compat;
pub mod prompt;
pub mod schema;
pub mod validate;

use std::sync::Arc;

use async_trait::async_trait;

use crate::config::{ExtractionConfig, ProviderKind};
use schema::Extraction;

/// What a provider returns: the parsed extraction plus metadata for evaluation.
#[derive(Debug, Clone)]
pub struct ProviderOutput {
    pub extraction: Extraction,
    pub input_tokens: Option<i32>,
    pub output_tokens: Option<i32>,
}

#[derive(Debug, thiserror::Error)]
pub enum ExtractError {
    /// HTTP 429 or quota exhausted. Retried later.
    #[error("rate limited: {0}")]
    RateLimited(String),
    /// Network errors, timeouts, 5xx. Retried later.
    #[error("transient: {0}")]
    Transient(String),
    /// Bad request, auth failure, unparseable output. Not retried.
    #[error("{0}")]
    Permanent(String),
}

impl ExtractError {
    pub fn is_retryable(&self) -> bool {
        !matches!(self, ExtractError::Permanent(_))
    }

    pub(crate) fn from_status(status: reqwest::StatusCode, body: &str) -> Self {
        let snippet: String = body.chars().take(300).collect();
        let msg = format!("HTTP {status}: {snippet}");
        if status.as_u16() == 429 {
            ExtractError::RateLimited(msg)
        } else if status.is_server_error() || status.as_u16() == 408 {
            ExtractError::Transient(msg)
        } else {
            ExtractError::Permanent(msg)
        }
    }

    pub(crate) fn from_reqwest(e: reqwest::Error) -> Self {
        if e.is_timeout() || e.is_connect() || e.is_request() {
            ExtractError::Transient(e.to_string())
        } else {
            ExtractError::Permanent(e.to_string())
        }
    }
}

#[async_trait]
pub trait ReceiptExtractor: Send + Sync {
    fn provider(&self) -> &'static str;
    fn model(&self) -> &str;
    async fn extract(&self, image: &[u8], mime: &str) -> Result<ProviderOutput, ExtractError>;
}

pub fn build_extractor(cfg: &ExtractionConfig) -> anyhow::Result<Arc<dyn ReceiptExtractor>> {
    let http = reqwest::Client::builder().timeout(cfg.timeout).build()?;
    Ok(match cfg.provider {
        ProviderKind::Gemini => {
            let g = cfg
                .gemini
                .clone()
                .expect("gemini config present when provider is gemini");
            Arc::new(gemini::GeminiExtractor::new(http, g))
        }
        ProviderKind::OpenAiCompatible => {
            let o = cfg
                .openai
                .clone()
                .expect("openai config present when provider is openai");
            Arc::new(openai_compat::OpenAiCompatExtractor::new(http, o))
        }
        ProviderKind::Mock => {
            tracing::warn!(
                "EXTRACTOR_PROVIDER=mock: receipts get fixture data, not real extraction"
            );
            Arc::new(mock::MockExtractor::from_path(cfg.mock_fixture.as_deref())?)
        }
    })
}

/// Pull the JSON object out of a model reply. Local models often wrap it in
/// ```json fences or add a sentence before it.
pub fn parse_model_json(text: &str) -> Result<Extraction, ExtractError> {
    let start = text.find('{');
    let end = text.rfind('}');
    let slice = match (start, end) {
        (Some(s), Some(e)) if e > s => &text[s..=e],
        _ => {
            return Err(ExtractError::Permanent(format!(
                "model reply contained no JSON object: {}",
                text.chars().take(200).collect::<String>()
            )));
        }
    };
    serde_json::from_str(slice)
        .map_err(|e| ExtractError::Permanent(format!("model JSON didn't match the schema: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_inside_markdown_fences() {
        let text = "Here is the receipt:\n```json\n{\"merchant\": {\"value\": \"Alfamart\", \"confidence\": 0.9}}\n```";
        let e = parse_model_json(text).unwrap();
        assert_eq!(e.merchant.value.as_deref(), Some("Alfamart"));
    }

    #[test]
    fn no_json_is_permanent_error() {
        let err = parse_model_json("I can't read this image.").unwrap_err();
        assert!(!err.is_retryable());
    }
}
