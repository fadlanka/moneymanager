use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, bail};

/// Which extraction backend the worker uses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderKind {
    Gemini,
    /// Any OpenAI-compatible chat completions endpoint: Ollama, LM Studio, vLLM, OpenRouter...
    OpenAiCompatible,
    /// Fixed fixture output. For tests and local UI work only.
    Mock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonMode {
    JsonSchema,
    JsonObject,
}

#[derive(Debug, Clone)]
pub struct GeminiConfig {
    pub api_key: String,
    pub model: String,
    /// `None` omits thinkingConfig entirely (for models that reject it).
    pub thinking_budget: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct OpenAiConfig {
    pub base_url: String,
    pub api_key: Option<String>,
    pub model: String,
    pub json_mode: JsonMode,
}

#[derive(Debug, Clone)]
pub struct ExtractionConfig {
    pub provider: ProviderKind,
    pub gemini: Option<GeminiConfig>,
    pub openai: Option<OpenAiConfig>,
    pub mock_fixture: Option<PathBuf>,
    pub requests_per_minute: u32,
    pub timeout: Duration,
    pub max_attempts: i32,
    pub poll_interval: Duration,
    pub review_threshold: f32,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub bind_addr: String,
    pub jwt_secret: String,
    pub invite_code: Option<String>,
    pub upload_dir: PathBuf,
    pub max_upload_bytes: usize,
    /// Allowed browser origins for the web (PWA) build. Empty = no CORS headers.
    pub cors_origins: Vec<String>,
    pub extraction: ExtractionConfig,
}

fn var(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.trim().is_empty())
}

fn required(key: &str) -> anyhow::Result<String> {
    var(key).with_context(|| format!("missing required env var {key}"))
}

fn parsed<T: std::str::FromStr>(key: &str, default: T) -> anyhow::Result<T>
where
    T::Err: std::fmt::Display,
{
    match var(key) {
        None => Ok(default),
        Some(raw) => raw
            .parse()
            .map_err(|e| anyhow::anyhow!("invalid value for {key}: {e}")),
    }
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let jwt_secret = required("JWT_SECRET")?;
        if jwt_secret.len() < 32 {
            bail!("JWT_SECRET must be at least 32 characters");
        }

        let provider = match required("EXTRACTOR_PROVIDER")?.to_lowercase().as_str() {
            "gemini" => ProviderKind::Gemini,
            "openai" | "ollama" => ProviderKind::OpenAiCompatible,
            "mock" => ProviderKind::Mock,
            other => bail!("EXTRACTOR_PROVIDER must be gemini, ollama/openai or mock, got {other}"),
        };

        let gemini = if provider == ProviderKind::Gemini {
            Some(GeminiConfig {
                api_key: required("GEMINI_API_KEY")?,
                model: var("GEMINI_MODEL").unwrap_or_else(|| "gemini-2.5-flash".into()),
                thinking_budget: match std::env::var("GEMINI_THINKING_BUDGET") {
                    Err(_) => Some(0),
                    Ok(v) if v.trim().is_empty() => None,
                    Ok(v) => Some(v.trim().parse().context("GEMINI_THINKING_BUDGET")?),
                },
            })
        } else {
            None
        };

        let openai = if provider == ProviderKind::OpenAiCompatible {
            Some(OpenAiConfig {
                base_url: var("OPENAI_BASE_URL")
                    .unwrap_or_else(|| "http://localhost:11434/v1".into())
                    .trim_end_matches('/')
                    .to_string(),
                api_key: var("OPENAI_API_KEY"),
                model: var("OPENAI_MODEL").unwrap_or_else(|| "qwen2.5vl:7b".into()),
                json_mode: match var("OPENAI_JSON_MODE").as_deref() {
                    None | Some("json_object") => JsonMode::JsonObject,
                    Some("json_schema") => JsonMode::JsonSchema,
                    Some(other) => {
                        bail!("OPENAI_JSON_MODE must be json_object or json_schema, got {other}")
                    }
                },
            })
        } else {
            None
        };

        let rpm: u32 = parsed("LLM_REQUESTS_PER_MINUTE", 10)?;
        if rpm == 0 {
            bail!("LLM_REQUESTS_PER_MINUTE must be > 0");
        }

        Ok(Self {
            database_url: required("DATABASE_URL")?,
            bind_addr: var("BIND_ADDR").unwrap_or_else(|| "0.0.0.0:8080".into()),
            jwt_secret,
            invite_code: var("INVITE_CODE"),
            upload_dir: PathBuf::from(var("UPLOAD_DIR").unwrap_or_else(|| "./data/uploads".into())),
            max_upload_bytes: parsed::<usize>("MAX_UPLOAD_MB", 10)? * 1024 * 1024,
            cors_origins: var("CORS_ORIGINS")
                .map(|v| {
                    v.split(',')
                        .map(|s| s.trim().trim_end_matches('/').to_string())
                        .filter(|s| !s.is_empty())
                        .collect()
                })
                .unwrap_or_default(),
            extraction: ExtractionConfig {
                provider,
                gemini,
                openai,
                mock_fixture: var("MOCK_FIXTURE").map(PathBuf::from),
                requests_per_minute: rpm,
                timeout: Duration::from_secs(parsed("LLM_TIMEOUT_SECS", 180)?),
                max_attempts: parsed("MAX_ATTEMPTS", 3)?,
                poll_interval: Duration::from_secs(parsed("WORKER_POLL_SECS", 3)?),
                review_threshold: parsed("REVIEW_CONFIDENCE_THRESHOLD", 0.75)?,
            },
        })
    }
}
