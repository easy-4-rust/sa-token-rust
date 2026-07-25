//! Runtime-scoped API Key plugin.

use std::sync::Arc;

use async_trait::async_trait;
use sa_token_adapter::SaStorage;
use sa_token_core::{SaTokenPlugin, SaTokenResult, SaTokenRuntime};

pub use sa_token_core::api_security::{ApiKeyConfig, ApiKeyModel, ApiKeyTemplate};

pub const APIKEY_PLUGIN_NAME: &str = "apikey";

/// Installs [`ApiKeyTemplate`] into one runtime's type-safe extension registry.
pub struct ApiKeyPlugin {
    template: Arc<ApiKeyTemplate>,
}

impl ApiKeyPlugin {
    pub fn new(storage: Arc<dyn SaStorage>, key_prefix: impl Into<String>) -> Self {
        Self {
            template: Arc::new(ApiKeyTemplate::new(storage, key_prefix)),
        }
    }

    pub fn from_template(template: Arc<ApiKeyTemplate>) -> Self {
        Self { template }
    }

    pub fn template(&self) -> &Arc<ApiKeyTemplate> {
        &self.template
    }
}

#[async_trait]
impl SaTokenPlugin for ApiKeyPlugin {
    fn name(&self) -> &'static str {
        APIKEY_PLUGIN_NAME
    }

    async fn install(&self, runtime: &SaTokenRuntime) -> SaTokenResult<()> {
        runtime.insert_extension(self.template.clone())
    }

    async fn destroy(&self, runtime: &SaTokenRuntime) -> SaTokenResult<()> {
        runtime.remove_extension::<ApiKeyTemplate>()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sa_token_core::{SaTokenConfig, SaTokenManager};
    use sa_token_storage_memory::MemoryStorage;

    #[tokio::test]
    async fn install_and_destroy_are_runtime_scoped() {
        let storage = Arc::new(MemoryStorage::new());
        let runtime = SaTokenRuntime::new(SaTokenManager::new(
            storage.clone(),
            SaTokenConfig::default(),
        ));

        runtime
            .install_plugin(ApiKeyPlugin::new(storage, "satoken:"))
            .await
            .unwrap();
        let template = runtime.extension::<ApiKeyTemplate>().unwrap().unwrap();
        let key = template.create_api_key("user-1", 60).await.unwrap();
        assert_eq!(
            template
                .get_login_id_by_api_key(&key.api_key)
                .await
                .unwrap(),
            "user-1"
        );

        runtime.destroy_plugin(APIKEY_PLUGIN_NAME).await.unwrap();
        assert!(runtime.extension::<ApiKeyTemplate>().unwrap().is_none());
    }
}
