//! Opt-in observability components for Sa-Token.
//!
//! This crate never installs a process-global tracing subscriber. The
//! application owns subscriber composition, exporters and access control.

use std::future::Future;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use sa_token_adapter::storage::{Expiration, SaStorage, ScanPage, StorageResult, TtlState};
use sa_token_core::event::SaTokenListener;
use sa_token_core::{SaTokenPlugin, SaTokenResult, SaTokenRuntime};
use tracing::Instrument;

pub const EVENT_TOTAL: &str = "sa_token_event_total";

/// Runtime-local configuration and controls published by the plugin.
#[derive(Clone)]
pub struct ObservabilityState {
    service_name: Arc<str>,
    #[cfg(feature = "reload")]
    filter_control: Option<FilterControl>,
}

impl ObservabilityState {
    pub fn service_name(&self) -> &str {
        &self.service_name
    }

    #[cfg(feature = "reload")]
    pub fn filter_control(&self) -> Option<&FilterControl> {
        self.filter_control.as_ref()
    }
}

/// Installs only runtime-local observability state.
///
/// Subscriber layers remain application-owned, which prevents libraries from
/// racing to set the global default.
pub struct ObservabilityPlugin {
    state: ObservabilityState,
}

impl ObservabilityPlugin {
    pub fn new(service_name: impl Into<Arc<str>>) -> Self {
        Self {
            state: ObservabilityState {
                service_name: service_name.into(),
                #[cfg(feature = "reload")]
                filter_control: None,
            },
        }
    }

    #[cfg(feature = "reload")]
    pub fn with_filter_control(mut self, control: FilterControl) -> Self {
        self.state.filter_control = Some(control);
        self
    }
}

#[async_trait]
impl SaTokenPlugin for ObservabilityPlugin {
    fn name(&self) -> &'static str {
        "observability"
    }

    async fn install(&self, runtime: &SaTokenRuntime) -> SaTokenResult<()> {
        runtime.insert_extension(Arc::new(self.state.clone()))
    }

    async fn destroy(&self, runtime: &SaTokenRuntime) -> SaTokenResult<()> {
        runtime.remove_extension::<ObservabilityState>()?;
        Ok(())
    }
}

/// A storage decorator that emits one safe span and latency metric per call.
///
/// Keys, values and errors are never attached. `backend` must be a bounded
/// deployment label such as `memory`, `redis` or `postgres`.
pub struct ObservedStorage {
    backend: &'static str,
    inner: Arc<dyn SaStorage>,
}

impl ObservedStorage {
    pub fn new(backend: &'static str, inner: Arc<dyn SaStorage>) -> Self {
        Self { backend, inner }
    }

    pub fn into_inner(self) -> Arc<dyn SaStorage> {
        self.inner
    }

    async fn observe<T, F>(&self, operation: &'static str, future: F) -> StorageResult<T>
    where
        F: Future<Output = StorageResult<T>>,
    {
        let started = Instant::now();
        let span = tracing::debug_span!(
            "sa_token.storage",
            backend = self.backend,
            operation,
            outcome = tracing::field::Empty
        );
        let result = future.instrument(span.clone()).await;
        let outcome = if result.is_ok() { "success" } else { "error" };
        span.record("outcome", outcome);
        #[cfg(feature = "metrics")]
        metrics::histogram!(
            sa_token_core::telemetry::metrics::STORAGE_DURATION_SECONDS,
            "backend" => self.backend,
            "operation" => operation,
            "outcome" => outcome
        )
        .record(started.elapsed().as_secs_f64());
        #[cfg(not(feature = "metrics"))]
        let _ = started;
        result
    }
}

#[async_trait]
impl SaStorage for ObservedStorage {
    async fn get(&self, key: &str) -> StorageResult<Option<String>> {
        self.observe("get", self.inner.get(key)).await
    }

    async fn set(&self, key: &str, value: &str, ttl: Option<Duration>) -> StorageResult<()> {
        self.observe("set", self.inner.set(key, value, ttl)).await
    }

    async fn set_with_expiration(
        &self,
        key: &str,
        value: &str,
        expiration: Expiration,
    ) -> StorageResult<()> {
        self.observe(
            "set_with_expiration",
            self.inner.set_with_expiration(key, value, expiration),
        )
        .await
    }

    async fn set_if_absent(
        &self,
        key: &str,
        value: &str,
        expiration: Expiration,
    ) -> StorageResult<bool> {
        self.observe(
            "set_if_absent",
            self.inner.set_if_absent(key, value, expiration),
        )
        .await
    }

    async fn delete(&self, key: &str) -> StorageResult<()> {
        self.observe("delete", self.inner.delete(key)).await
    }

    async fn exists(&self, key: &str) -> StorageResult<bool> {
        self.observe("exists", self.inner.exists(key)).await
    }

    async fn expire(&self, key: &str, ttl: Duration) -> StorageResult<()> {
        self.observe("expire", self.inner.expire(key, ttl)).await
    }

    async fn ttl(&self, key: &str) -> StorageResult<Option<Duration>> {
        self.observe("ttl", self.inner.ttl(key)).await
    }

    async fn ttl_state(&self, key: &str) -> StorageResult<TtlState> {
        self.observe("ttl_state", self.inner.ttl_state(key)).await
    }

    async fn mget(&self, keys: &[&str]) -> StorageResult<Vec<Option<String>>> {
        self.observe("mget", self.inner.mget(keys)).await
    }

    async fn mset(&self, items: &[(&str, &str)], ttl: Option<Duration>) -> StorageResult<()> {
        self.observe("mset", self.inner.mset(items, ttl)).await
    }

    async fn mdel(&self, keys: &[&str]) -> StorageResult<()> {
        self.observe("mdel", self.inner.mdel(keys)).await
    }

    async fn increment_by(&self, key: &str, delta: i64) -> StorageResult<i64> {
        self.observe("increment_by", self.inner.increment_by(key, delta))
            .await
    }

    async fn clear(&self) -> StorageResult<()> {
        self.observe("clear", self.inner.clear()).await
    }

    async fn keys(&self, pattern: &str) -> StorageResult<Vec<String>> {
        self.observe("keys", self.inner.keys(pattern)).await
    }

    async fn scan(
        &self,
        pattern: &str,
        cursor: Option<&str>,
        limit: usize,
    ) -> StorageResult<ScanPage> {
        self.observe("scan", self.inner.scan(pattern, cursor, limit))
            .await
    }
}

/// Metrics-only event listener. No token, login ID, device or login type is
/// exported as a metric label.
#[derive(Default)]
pub struct MetricsListener;

impl MetricsListener {
    #[cfg(feature = "metrics")]
    fn record(event: &'static str) {
        metrics::counter!(EVENT_TOTAL, "event" => event).increment(1);
    }

    #[cfg(not(feature = "metrics"))]
    fn record(_event: &'static str) {}
}

#[async_trait]
impl SaTokenListener for MetricsListener {
    async fn on_login(&self, _: &str, _: &str, _: &str) {
        Self::record("login");
    }

    async fn on_logout(&self, _: &str, _: &str, _: &str) {
        Self::record("logout");
    }

    async fn on_kick_out(&self, _: &str, _: &str, _: &str) {
        Self::record("kick_out");
    }

    async fn on_renew_timeout(&self, _: &str, _: &str, _: &str) {
        Self::record("renew_timeout");
    }

    async fn on_replaced(&self, _: &str, _: &str, _: &str) {
        Self::record("replaced");
    }

    async fn on_banned(&self, _: &str, _: &str) {
        Self::record("banned");
    }

    async fn on_open_safe(&self, _: &str, _: &str) {
        Self::record("open_safe");
    }

    async fn on_close_safe(&self, _: &str, _: &str) {
        Self::record("close_safe");
    }
}

#[cfg(feature = "reload")]
#[derive(Clone)]
pub struct FilterControl {
    handle: tracing_subscriber::reload::Handle<
        tracing_subscriber::EnvFilter,
        tracing_subscriber::Registry,
    >,
}

#[cfg(feature = "reload")]
impl FilterControl {
    pub fn reload(
        &self,
        filter: impl AsRef<str>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let filter = tracing_subscriber::EnvFilter::try_new(filter.as_ref())?;
        self.handle.reload(filter)?;
        Ok(())
    }
}

/// Build an application-composable, reloadable filter layer.
#[cfg(feature = "reload")]
pub fn reloadable_env_filter(
    initial: impl AsRef<str>,
) -> Result<
    (
        tracing_subscriber::reload::Layer<
            tracing_subscriber::EnvFilter,
            tracing_subscriber::Registry,
        >,
        FilterControl,
    ),
    tracing_subscriber::filter::ParseError,
> {
    let filter = tracing_subscriber::EnvFilter::try_new(initial.as_ref())?;
    let (layer, handle) = tracing_subscriber::reload::Layer::new(filter);
    Ok((layer, FilterControl { handle }))
}

/// Re-export the official bridge so the application can compose its own OTel
/// layer and exporter without this library setting a global subscriber.
#[cfg(feature = "otel")]
pub use tracing_opentelemetry;

#[cfg(test)]
mod tests {
    use super::*;
    use sa_token_core::{SaTokenConfig, SaTokenManager};
    use sa_token_storage_memory::MemoryStorage;

    fn runtime() -> SaTokenRuntime {
        SaTokenRuntime::new(SaTokenManager::new(
            Arc::new(MemoryStorage::new()),
            SaTokenConfig::default(),
        ))
    }

    #[tokio::test]
    async fn observed_storage_delegates_real_operations() {
        let storage = ObservedStorage::new("memory", Arc::new(MemoryStorage::new()));
        storage
            .set("secret-key", "secret-value", None)
            .await
            .unwrap();
        assert_eq!(
            storage.get("secret-key").await.unwrap().as_deref(),
            Some("secret-value")
        );
        storage.delete("secret-key").await.unwrap();
        assert!(!storage.exists("secret-key").await.unwrap());
    }

    #[tokio::test]
    async fn plugin_lifecycle_is_runtime_scoped_and_reversible() {
        let runtime = runtime();
        runtime
            .install_plugin(ObservabilityPlugin::new("test-service"))
            .await
            .unwrap();
        let state = runtime.extension::<ObservabilityState>().unwrap().unwrap();
        assert_eq!(state.service_name(), "test-service");
        runtime.destroy_plugin("observability").await.unwrap();
        assert!(runtime.extension::<ObservabilityState>().unwrap().is_none());
    }

    #[cfg(feature = "metrics")]
    #[test]
    fn observed_storage_records_real_metrics_without_secret_labels() {
        let recorder = metrics_util::debugging::DebuggingRecorder::new();
        let snapshotter = recorder.snapshotter();
        metrics::with_local_recorder(&recorder, || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async {
                    let storage = ObservedStorage::new("memory", Arc::new(MemoryStorage::new()));
                    storage
                        .set("metric-secret-key", "metric-secret-value", None)
                        .await
                        .unwrap();
                    storage.get("metric-secret-key").await.unwrap();
                });
        });

        let snapshot = format!("{:?}", snapshotter.snapshot().into_vec());
        assert!(
            snapshot.contains("sa_token_storage_duration_seconds"),
            "{snapshot}"
        );
        assert!(
            snapshot.contains("Label(\"operation\", \"set\")"),
            "{snapshot}"
        );
        assert!(
            snapshot.contains("Label(\"operation\", \"get\")"),
            "{snapshot}"
        );
        assert!(!snapshot.contains("metric-secret-key"), "{snapshot}");
        assert!(!snapshot.contains("metric-secret-value"), "{snapshot}");
    }

    #[cfg(feature = "reload")]
    #[test]
    fn filter_can_be_reloaded_without_global_subscriber() {
        let (_layer, control) = reloadable_env_filter("info").unwrap();
        control.reload("sa_token_core=trace,warn").unwrap();
    }
}
