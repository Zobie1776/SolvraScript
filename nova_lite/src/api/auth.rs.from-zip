#![allow(dead_code)]

use std::sync::Arc;
use std::time::SystemTime;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::warn;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<SystemTime>,
}

impl AuthToken {
    pub fn is_expired(&self) -> bool {
        self.expires_at
            .map(|expires| SystemTime::now() >= expires)
            .unwrap_or(false)
    }
}

#[async_trait]
pub trait AuthProvider: Send + Sync {
    async fn authenticate(&self) -> anyhow::Result<AuthToken>;
    async fn refresh(&self, token: &AuthToken) -> anyhow::Result<AuthToken>;
}

#[derive(Clone)]
pub struct AuthState {
    provider: Option<Arc<dyn AuthProvider>>,
    token: Arc<RwLock<Option<AuthToken>>>,
}

impl AuthState {
    pub fn new(provider: Option<Arc<dyn AuthProvider>>) -> Self {
        Self {
            provider,
            token: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn token(&self) -> anyhow::Result<Option<AuthToken>> {
        let mut guard = self.token.write().await;
        if let Some(ref provider) = self.provider {
            if guard.as_ref().map(|t| t.is_expired()).unwrap_or(true) {
                let next = if let Some(current) = guard.clone() {
                    match provider.refresh(&current).await {
                        Ok(token) => token,
                        Err(err) => {
                            warn!("failed to refresh token: {err:?}");
                            current
                        }
                    }
                } else {
                    provider.authenticate().await?
                };
                *guard = Some(next);
            }
        }
        Ok(guard.clone())
    }
}

pub struct StaticAuthProvider {
    token: AuthToken,
}

impl StaticAuthProvider {
    pub fn new(token: AuthToken) -> Self {
        Self { token }
    }
}

#[async_trait]
impl AuthProvider for StaticAuthProvider {
    async fn authenticate(&self) -> anyhow::Result<AuthToken> {
        Ok(self.token.clone())
    }

    async fn refresh(&self, _token: &AuthToken) -> anyhow::Result<AuthToken> {
        Ok(self.token.clone())
    }
}
