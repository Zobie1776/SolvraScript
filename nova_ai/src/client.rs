use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiCompletion {
    pub title: String,
    pub body: String,
}

#[derive(Debug, Error)]
pub enum NovaAiError {
    #[error("NovaAI backend not configured")]
    NotConfigured,
}

#[async_trait]
pub trait NovaAiService: Send + Sync {
    async fn complete(&self, prompt: &str) -> Result<AiCompletion, NovaAiError>;
}

#[derive(Debug, Default)]
pub struct MockService;

#[async_trait]
impl NovaAiService for MockService {
    async fn complete(&self, prompt: &str) -> Result<AiCompletion, NovaAiError> {
        Ok(AiCompletion {
            title: "Mock Completion".into(),
            body: format!("Echo: {prompt}"),
        })
    }
}
