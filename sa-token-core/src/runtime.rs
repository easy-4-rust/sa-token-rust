//! Explicit, isolated Sa-Token runtime.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::{Arc, RwLock};

use tokio::sync::Mutex;
use tracing::Instrument;

use crate::{SaTokenError, SaTokenManager, SaTokenPlugin, SaTokenResult, StpUtil};

enum PluginSlot {
    Installing,
    Installed(Arc<dyn SaTokenPlugin>),
    Destroying(Arc<dyn SaTokenPlugin>),
}

#[derive(Default)]
struct RuntimeRegistry {
    extensions: RwLock<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>,
    plugins: Mutex<HashMap<&'static str, PluginSlot>>,
}

/// Owns one isolated authentication runtime.
///
/// Building a runtime does not mutate process-global state. Applications that
/// need the legacy [`StpUtil`] facade must install it explicitly.
#[derive(Clone)]
pub struct SaTokenRuntime {
    manager: Arc<SaTokenManager>,
    registry: Arc<RuntimeRegistry>,
}

impl SaTokenRuntime {
    pub fn new(manager: SaTokenManager) -> Self {
        Self {
            manager: Arc::new(manager),
            registry: Arc::new(RuntimeRegistry::default()),
        }
    }

    pub fn from_manager(manager: Arc<SaTokenManager>) -> Self {
        Self {
            manager,
            registry: Arc::new(RuntimeRegistry::default()),
        }
    }

    pub fn manager(&self) -> &Arc<SaTokenManager> {
        &self.manager
    }

    /// Explicitly install this runtime for the legacy global [`StpUtil`] facade.
    pub fn install_global(&self) -> SaTokenResult<()> {
        StpUtil::init_manager_arc(self.manager.clone())
    }

    /// Publish one type-safe, runtime-local extension.
    pub fn insert_extension<T>(&self, extension: Arc<T>) -> SaTokenResult<()>
    where
        T: Send + Sync + 'static,
    {
        let mut extensions = self.registry.extensions.write().map_err(|_| {
            SaTokenError::PluginError("runtime extension registry lock poisoned".to_string())
        })?;
        let type_id = TypeId::of::<T>();
        if extensions.contains_key(&type_id) {
            return Err(SaTokenError::PluginError(format!(
                "runtime extension `{}` is already installed",
                std::any::type_name::<T>()
            )));
        }
        extensions.insert(type_id, extension);
        Ok(())
    }

    /// Resolve a type-safe extension from this runtime.
    pub fn extension<T>(&self) -> SaTokenResult<Option<Arc<T>>>
    where
        T: Send + Sync + 'static,
    {
        let extensions = self.registry.extensions.read().map_err(|_| {
            SaTokenError::PluginError("runtime extension registry lock poisoned".to_string())
        })?;
        Ok(extensions
            .get(&TypeId::of::<T>())
            .cloned()
            .and_then(|extension| extension.downcast::<T>().ok()))
    }

    /// Remove and return one type-safe extension.
    pub fn remove_extension<T>(&self) -> SaTokenResult<Option<Arc<T>>>
    where
        T: Send + Sync + 'static,
    {
        let mut extensions = self.registry.extensions.write().map_err(|_| {
            SaTokenError::PluginError("runtime extension registry lock poisoned".to_string())
        })?;
        Ok(extensions
            .remove(&TypeId::of::<T>())
            .and_then(|extension| extension.downcast::<T>().ok()))
    }

    /// Install a plugin exactly once in this runtime.
    ///
    /// The name is reserved before invoking the hook, so concurrent duplicate
    /// installation cannot run the hook twice. A failed hook releases the
    /// reservation and may be retried.
    pub async fn install_plugin<P>(&self, plugin: P) -> SaTokenResult<()>
    where
        P: SaTokenPlugin,
    {
        self.install_plugin_arc(Arc::new(plugin)).await
    }

    /// Install an already shared plugin instance.
    pub async fn install_plugin_arc(&self, plugin: Arc<dyn SaTokenPlugin>) -> SaTokenResult<()> {
        let name = plugin.name();
        let span = tracing::info_span!(
            "sa_token.plugin.lifecycle",
            plugin = name,
            action = "install",
            outcome = tracing::field::Empty
        );
        {
            let mut plugins = self.registry.plugins.lock().await;
            if plugins.contains_key(name) {
                crate::telemetry::record_plugin_lifecycle(name, "install", "duplicate");
                span.record("outcome", "duplicate");
                return Err(SaTokenError::PluginError(format!(
                    "plugin `{name}` is already installed or changing state"
                )));
            }
            plugins.insert(name, PluginSlot::Installing);
        }

        if let Err(error) = plugin.install(self).instrument(span.clone()).await {
            self.registry.plugins.lock().await.remove(name);
            crate::telemetry::record_plugin_lifecycle(name, "install", "error");
            span.record("outcome", "error");
            return Err(error);
        }

        self.registry
            .plugins
            .lock()
            .await
            .insert(name, PluginSlot::Installed(plugin));
        crate::telemetry::record_plugin_lifecycle(name, "install", "success");
        span.record("outcome", "success");
        Ok(())
    }

    /// Destroy an installed plugin.
    ///
    /// A failed destroy hook rolls the lifecycle state back to `Installed` so
    /// callers can retry without losing the plugin registration.
    pub async fn destroy_plugin(&self, name: &str) -> SaTokenResult<()> {
        let span = tracing::info_span!(
            "sa_token.plugin.lifecycle",
            plugin = name,
            action = "destroy",
            outcome = tracing::field::Empty
        );
        let plugin = {
            let mut plugins = self.registry.plugins.lock().await;
            match plugins.get(name) {
                Some(PluginSlot::Installed(plugin)) => {
                    let plugin = plugin.clone();
                    plugins.insert(plugin.name(), PluginSlot::Destroying(plugin.clone()));
                    plugin
                }
                Some(PluginSlot::Installing) | Some(PluginSlot::Destroying(_)) => {
                    span.record("outcome", "busy");
                    return Err(SaTokenError::PluginError(format!(
                        "plugin `{name}` is changing state"
                    )));
                }
                None => {
                    span.record("outcome", "missing");
                    return Err(SaTokenError::PluginError(format!(
                        "plugin `{name}` is not installed"
                    )));
                }
            }
        };

        if let Err(error) = plugin.destroy(self).instrument(span.clone()).await {
            let plugin_name = plugin.name();
            self.registry
                .plugins
                .lock()
                .await
                .insert(plugin_name, PluginSlot::Installed(plugin));
            crate::telemetry::record_plugin_lifecycle(plugin_name, "destroy", "error");
            span.record("outcome", "error");
            return Err(error);
        }

        let mut plugins = self.registry.plugins.lock().await;
        if matches!(
            plugins.get(plugin.name()),
            Some(PluginSlot::Destroying(current)) if Arc::ptr_eq(current, &plugin)
        ) {
            plugins.remove(plugin.name());
        }
        crate::telemetry::record_plugin_lifecycle(plugin.name(), "destroy", "success");
        span.record("outcome", "success");
        Ok(())
    }

    /// Sorted names of fully installed plugins.
    pub async fn installed_plugins(&self) -> Vec<&'static str> {
        let plugins = self.registry.plugins.lock().await;
        let mut names = plugins
            .iter()
            .filter_map(|(name, slot)| matches!(slot, PluginSlot::Installed(_)).then_some(*name))
            .collect::<Vec<_>>();
        names.sort_unstable();
        names
    }
}

impl Deref for SaTokenRuntime {
    type Target = SaTokenManager;

    fn deref(&self) -> &Self::Target {
        &self.manager
    }
}

impl From<SaTokenManager> for SaTokenRuntime {
    fn from(manager: SaTokenManager) -> Self {
        Self::new(manager)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    use async_trait::async_trait;
    use sa_token_storage_memory::MemoryStorage;

    use super::*;
    use crate::{SaTokenConfig, SaTokenPlugin};

    struct Marker(&'static str);

    struct TestPlugin {
        installs: Arc<AtomicUsize>,
        destroys: Arc<AtomicUsize>,
        fail_destroy: Arc<AtomicBool>,
    }

    #[async_trait]
    impl SaTokenPlugin for TestPlugin {
        fn name(&self) -> &'static str {
            "test"
        }

        async fn install(&self, runtime: &SaTokenRuntime) -> SaTokenResult<()> {
            self.installs.fetch_add(1, Ordering::SeqCst);
            runtime.insert_extension(Arc::new(Marker("installed")))
        }

        async fn destroy(&self, runtime: &SaTokenRuntime) -> SaTokenResult<()> {
            self.destroys.fetch_add(1, Ordering::SeqCst);
            if self.fail_destroy.load(Ordering::SeqCst) {
                return Err(SaTokenError::PluginError(
                    "expected destroy failure".to_string(),
                ));
            }
            runtime.remove_extension::<Marker>()?;
            Ok(())
        }
    }

    fn runtime() -> SaTokenRuntime {
        SaTokenRuntime::new(SaTokenManager::new(
            Arc::new(MemoryStorage::new()),
            SaTokenConfig::default(),
        ))
    }

    #[tokio::test]
    async fn two_runtimes_are_isolated_without_global_installation() {
        let first = runtime();
        let second = runtime();

        let token = first.login("user-1").await.unwrap();

        assert!(first.is_valid(&token).await);
        assert!(!second.is_valid(&token).await);
    }

    #[tokio::test]
    async fn plugin_lifecycle_is_runtime_scoped_and_rejects_duplicates() {
        let first = runtime();
        let second = runtime();
        let installs = Arc::new(AtomicUsize::new(0));
        let destroys = Arc::new(AtomicUsize::new(0));
        let fail_destroy = Arc::new(AtomicBool::new(false));

        first
            .install_plugin(TestPlugin {
                installs: installs.clone(),
                destroys: destroys.clone(),
                fail_destroy: fail_destroy.clone(),
            })
            .await
            .unwrap();

        assert_eq!(first.installed_plugins().await, vec!["test"]);
        assert_eq!(first.extension::<Marker>().unwrap().unwrap().0, "installed");
        assert!(second.extension::<Marker>().unwrap().is_none());
        assert!(
            first
                .install_plugin(TestPlugin {
                    installs: installs.clone(),
                    destroys: destroys.clone(),
                    fail_destroy: fail_destroy.clone(),
                })
                .await
                .is_err()
        );
        assert_eq!(installs.load(Ordering::SeqCst), 1);

        first.destroy_plugin("test").await.unwrap();
        assert!(first.installed_plugins().await.is_empty());
        assert!(first.extension::<Marker>().unwrap().is_none());
        assert_eq!(destroys.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn failed_destroy_rolls_back_plugin_state() {
        let runtime = runtime();
        let fail_destroy = Arc::new(AtomicBool::new(true));
        runtime
            .install_plugin(TestPlugin {
                installs: Arc::new(AtomicUsize::new(0)),
                destroys: Arc::new(AtomicUsize::new(0)),
                fail_destroy: fail_destroy.clone(),
            })
            .await
            .unwrap();

        assert!(runtime.destroy_plugin("test").await.is_err());
        assert_eq!(runtime.installed_plugins().await, vec!["test"]);
        assert!(runtime.extension::<Marker>().unwrap().is_some());

        fail_destroy.store(false, Ordering::SeqCst);
        runtime.destroy_plugin("test").await.unwrap();
        assert!(runtime.installed_plugins().await.is_empty());
    }
}
