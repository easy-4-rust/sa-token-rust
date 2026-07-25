use std::sync::Arc;

use sa_token_core::{SaTokenConfig, SaTokenError, StpUtil};
use sa_token_storage_memory::MemoryStorage;

#[tokio::test]
async fn builder_is_isolated_until_runtime_is_explicitly_installed() {
    assert!(!StpUtil::is_initialized());

    let runtime = SaTokenConfig::builder()
        .storage(Arc::new(MemoryStorage::new()))
        .build()
        .expect("build runtime");

    assert!(!StpUtil::is_initialized());
    let token = runtime.login("isolated-user").await.expect("login");
    assert!(runtime.is_valid(&token).await);

    runtime.install_global().expect("install global facade");
    assert!(StpUtil::is_initialized());
    assert_eq!(
        StpUtil::get_login_id(&token).await.expect("global lookup"),
        "isolated-user"
    );
}

#[test]
fn builder_without_storage_returns_config_error() {
    let result = SaTokenConfig::builder().build();
    assert!(matches!(result, Err(SaTokenError::ConfigError(_))));
}
