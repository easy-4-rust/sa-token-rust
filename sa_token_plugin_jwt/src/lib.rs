//! Runtime-scoped JWT plugin.

use std::sync::Arc;

use async_trait::async_trait;
use sa_token_core::{SaTokenError, SaTokenPlugin, SaTokenResult, SaTokenRuntime};

pub use sa_token_core::token::{JwtAlgorithm, JwtClaims, JwtManager};

pub const JWT_PLUGIN_NAME: &str = "jwt";

/// Installs one validated [`JwtManager`] into a runtime.
pub struct JwtPlugin {
    manager: Arc<JwtManager>,
}

impl JwtPlugin {
    pub fn new(secret: impl Into<String>) -> SaTokenResult<Self> {
        let secret = secret.into();
        if secret.trim().is_empty() {
            return Err(SaTokenError::ConfigError(
                "JWT secret must not be empty".to_string(),
            ));
        }
        Ok(Self::from_manager(Arc::new(JwtManager::new(secret))))
    }

    pub fn with_algorithm(
        secret: impl Into<String>,
        algorithm: JwtAlgorithm,
    ) -> SaTokenResult<Self> {
        let secret = secret.into();
        if secret.trim().is_empty() {
            return Err(SaTokenError::ConfigError(
                "JWT secret must not be empty".to_string(),
            ));
        }
        Ok(Self::from_manager(Arc::new(JwtManager::with_algorithm(
            secret, algorithm,
        ))))
    }

    pub fn from_manager(manager: Arc<JwtManager>) -> Self {
        Self { manager }
    }

    pub fn manager(&self) -> &Arc<JwtManager> {
        &self.manager
    }
}

#[async_trait]
impl SaTokenPlugin for JwtPlugin {
    fn name(&self) -> &'static str {
        JWT_PLUGIN_NAME
    }

    async fn install(&self, runtime: &SaTokenRuntime) -> SaTokenResult<()> {
        runtime.insert_extension(self.manager.clone())
    }

    async fn destroy(&self, runtime: &SaTokenRuntime) -> SaTokenResult<()> {
        runtime.remove_extension::<JwtManager>()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sa_token_core::{SaTokenConfig, SaTokenManager};
    use sa_token_storage_memory::MemoryStorage;

    #[tokio::test]
    async fn rejects_empty_secret_and_manages_lifecycle() {
        assert!(JwtPlugin::new("  ").is_err());
        let runtime = SaTokenRuntime::new(SaTokenManager::new(
            Arc::new(MemoryStorage::new()),
            SaTokenConfig::default(),
        ));

        runtime
            .install_plugin(JwtPlugin::new("secret-for-test").unwrap())
            .await
            .unwrap();
        assert!(runtime.extension::<JwtManager>().unwrap().is_some());

        runtime.destroy_plugin(JWT_PLUGIN_NAME).await.unwrap();
        assert!(runtime.extension::<JwtManager>().unwrap().is_none());
    }
}
