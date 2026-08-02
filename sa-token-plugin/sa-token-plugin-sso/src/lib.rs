//! Runtime-scoped SSO plugin.

use std::sync::Arc;

use async_trait::async_trait;
use sa_token_core::{SaTokenPlugin, SaTokenResult, SaTokenRuntime};

pub use sa_token_core::sso::{
    CheckTicketResult, SsoClient, SsoConfig, SsoManager, SsoServer, SsoSession, SsoTicket,
};

pub const SSO_PLUGIN_NAME: &str = "sso";

/// Installs one [`SsoManager`] into a runtime.
pub struct SsoPlugin {
    manager: Arc<SsoManager>,
}

impl SsoPlugin {
    pub fn new(config: SsoConfig) -> Self {
        Self::from_manager(Arc::new(SsoManager::new(config)))
    }

    pub fn from_manager(manager: Arc<SsoManager>) -> Self {
        Self { manager }
    }

    pub fn manager(&self) -> &Arc<SsoManager> {
        &self.manager
    }
}

#[async_trait]
impl SaTokenPlugin for SsoPlugin {
    fn name(&self) -> &'static str {
        SSO_PLUGIN_NAME
    }

    async fn install(&self, runtime: &SaTokenRuntime) -> SaTokenResult<()> {
        runtime.insert_extension(self.manager.clone())
    }

    async fn destroy(&self, runtime: &SaTokenRuntime) -> SaTokenResult<()> {
        runtime.remove_extension::<SsoManager>()?;
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
        let runtime = SaTokenRuntime::new(SaTokenManager::new(
            Arc::new(MemoryStorage::new()),
            SaTokenConfig::default(),
        ));

        runtime
            .install_plugin(SsoPlugin::new(SsoConfig::default()))
            .await
            .unwrap();
        assert!(runtime.extension::<SsoManager>().unwrap().is_some());

        runtime.destroy_plugin(SSO_PLUGIN_NAME).await.unwrap();
        assert!(runtime.extension::<SsoManager>().unwrap().is_none());
    }
}
