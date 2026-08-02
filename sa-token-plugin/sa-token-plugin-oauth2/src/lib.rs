//! Runtime-scoped OAuth2 plugin.

use std::sync::Arc;

use async_trait::async_trait;
use sa_token_adapter::SaStorage;
use sa_token_core::{SaTokenPlugin, SaTokenResult, SaTokenRuntime};

pub use sa_token_core::oauth2::{
    AccessToken, AuthorizationCode, OAuth2Client, OAuth2Manager, OAuth2TokenInfo,
};

pub const OAUTH2_PLUGIN_NAME: &str = "oauth2";

/// Installs one [`OAuth2Manager`] into a runtime.
pub struct OAuth2Plugin {
    manager: Arc<OAuth2Manager>,
}

impl OAuth2Plugin {
    pub fn new(storage: Arc<dyn SaStorage>) -> Self {
        Self::from_manager(Arc::new(OAuth2Manager::new(storage)))
    }

    pub fn from_manager(manager: Arc<OAuth2Manager>) -> Self {
        Self { manager }
    }

    pub fn manager(&self) -> &Arc<OAuth2Manager> {
        &self.manager
    }
}

#[async_trait]
impl SaTokenPlugin for OAuth2Plugin {
    fn name(&self) -> &'static str {
        OAUTH2_PLUGIN_NAME
    }

    async fn install(&self, runtime: &SaTokenRuntime) -> SaTokenResult<()> {
        runtime.insert_extension(self.manager.clone())
    }

    async fn destroy(&self, runtime: &SaTokenRuntime) -> SaTokenResult<()> {
        runtime.remove_extension::<OAuth2Manager>()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sa_token_core::{SaTokenConfig, SaTokenManager};
    use sa_token_storage_memory::MemoryStorage;

    #[tokio::test]
    async fn lifecycle_publishes_manager() {
        let storage = Arc::new(MemoryStorage::new());
        let runtime = SaTokenRuntime::new(SaTokenManager::new(
            storage.clone(),
            SaTokenConfig::default(),
        ));

        runtime
            .install_plugin(OAuth2Plugin::new(storage))
            .await
            .unwrap();
        assert!(runtime.extension::<OAuth2Manager>().unwrap().is_some());

        runtime.destroy_plugin(OAUTH2_PLUGIN_NAME).await.unwrap();
        assert!(runtime.extension::<OAuth2Manager>().unwrap().is_none());
    }
}
