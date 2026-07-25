//! Runtime-scoped plugin lifecycle.

use async_trait::async_trait;

use crate::{SaTokenResult, SaTokenRuntime};

/// A capability that can be installed into one [`SaTokenRuntime`].
///
/// Plugins are runtime-scoped: installing a plugin never mutates process-global
/// state, and two runtimes may install different plugin configurations.
#[async_trait]
pub trait SaTokenPlugin: Send + Sync + 'static {
    /// Stable lifecycle name used to reject duplicate installation.
    fn name(&self) -> &'static str;

    /// Install the plugin and publish its runtime extensions.
    async fn install(&self, runtime: &SaTokenRuntime) -> SaTokenResult<()>;

    /// Remove resources and extensions published by this plugin.
    async fn destroy(&self, _runtime: &SaTokenRuntime) -> SaTokenResult<()> {
        Ok(())
    }
}
