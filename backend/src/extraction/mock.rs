//! Fixture-backed extractor for tests and UI work without an API key.
//! It never looks at the image.

use std::path::Path;
use std::time::Duration;

use async_trait::async_trait;

use super::{ExtractError, ProviderOutput, ReceiptExtractor, parse_model_json};

const DEFAULT_FIXTURE: &str = include_str!("../../fixtures/indomaret.json");

pub struct MockExtractor {
    fixture: String,
    delay: Duration,
}

impl MockExtractor {
    pub fn new(fixture: impl Into<String>, delay: Duration) -> Self {
        Self {
            fixture: fixture.into(),
            delay,
        }
    }

    pub fn from_path(path: Option<&Path>) -> anyhow::Result<Self> {
        let fixture = match path {
            Some(p) => std::fs::read_to_string(p)?,
            None => DEFAULT_FIXTURE.to_string(),
        };
        // Fail at startup, not on the first receipt, if the fixture is broken.
        parse_model_json(&fixture).map_err(|e| anyhow::anyhow!("bad MOCK_FIXTURE: {e}"))?;
        // A short delay so the app's "reading receipt" state is visible.
        Ok(Self::new(fixture, Duration::from_millis(1500)))
    }
}

#[async_trait]
impl ReceiptExtractor for MockExtractor {
    fn provider(&self) -> &'static str {
        "mock"
    }

    fn model(&self) -> &str {
        "fixture"
    }

    async fn extract(&self, _image: &[u8], _mime: &str) -> Result<ProviderOutput, ExtractError> {
        tokio::time::sleep(self.delay).await;
        Ok(ProviderOutput {
            extraction: parse_model_json(&self.fixture)?,
            input_tokens: None,
            output_tokens: None,
        })
    }
}
