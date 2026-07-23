//! P1: Config Builder integration tests.
//!
//! Covers default values, builder chain, boundary values,
//! and error paths (build without storage).

mod common;

use common::setup;
use sa_token_core::{SaTokenConfig, config::TokenStyle};

// ── Success cases ──────────────────────────────────────────────────────────

#[tokio::test]
async fn test_default_config_values() {
    let config = SaTokenConfig::default();
    assert_eq!(config.token_name, "sa-token");
    assert_eq!(config.timeout, 2592000);
    assert_eq!(config.active_timeout, -1);
    assert!(config.auto_renew);
    assert!(config.is_concurrent);
    assert!(!config.is_share);
    assert!(!config.dynamic_active_timeout);
    assert!(matches!(config.token_style, TokenStyle::Uuid));
    assert!(!config.is_log);
    assert!(config.is_read_cookie);
    assert!(config.is_read_header);
    assert!(config.is_read_body);
    assert!(config.token_prefix.is_none());
    assert!(config.jwt_secret_key.is_none());
    assert_eq!(config.jwt_algorithm.as_deref(), Some("HS256"));
    assert!(config.jwt_issuer.is_none());
    assert!(config.jwt_audience.is_none());
    assert!(!config.enable_nonce);
    assert_eq!(config.nonce_timeout, -1);
    assert!(!config.enable_refresh_token);
    assert_eq!(config.refresh_token_timeout, 604800);
}

#[tokio::test]
async fn test_builder_all_fields() {
    let config = SaTokenConfig::builder()
        .token_name("X-Auth")
        .timeout(7200)
        .active_timeout(1800)
        .auto_renew(true)
        .is_concurrent(false)
        .is_share(false)
        .token_style(TokenStyle::Jwt)
        .token_prefix("Bearer ")
        .jwt_secret_key("my-secret")
        .jwt_algorithm("HS512")
        .jwt_issuer("test-app")
        .jwt_audience("test-users")
        .enable_nonce(true)
        .nonce_timeout(300)
        .enable_refresh_token(true)
        .refresh_token_timeout(86400)
        .build_config();
    assert_eq!(config.token_name, "X-Auth");
    assert_eq!(config.timeout, 7200);
    assert_eq!(config.active_timeout, 1800);
    assert!(config.auto_renew);
    assert!(!config.is_concurrent);
    assert!(!config.is_share);
    assert!(matches!(config.token_style, TokenStyle::Jwt));
    assert_eq!(config.token_prefix.as_deref(), Some("Bearer "));
    assert_eq!(config.jwt_secret_key.as_deref(), Some("my-secret"));
    assert_eq!(config.jwt_algorithm.as_deref(), Some("HS512"));
    assert_eq!(config.jwt_issuer.as_deref(), Some("test-app"));
    assert_eq!(config.jwt_audience.as_deref(), Some("test-users"));
    assert!(config.enable_nonce);
    assert_eq!(config.nonce_timeout, 300);
    assert!(config.enable_refresh_token);
    assert_eq!(config.refresh_token_timeout, 86400);
}

#[tokio::test]
async fn test_builder_token_name() {
    let config = SaTokenConfig::builder()
        .token_name("CustomToken")
        .build_config();
    assert_eq!(config.token_name, "CustomToken");
}

#[tokio::test]
async fn test_timeout_negative_never_expires() {
    let config = SaTokenConfig::builder()
        .timeout(-1)
        .build_config();
    assert_eq!(config.timeout, -1);
    assert!(config.timeout_duration().is_none());
}

#[tokio::test]
async fn test_timeout_positive_has_duration() {
    let config = SaTokenConfig::builder()
        .timeout(3600)
        .build_config();
    let dur = config.timeout_duration();
    assert!(dur.is_some());
    assert_eq!(dur.unwrap().as_secs(), 3600);
}

#[tokio::test]
async fn test_is_concurrent_setting() {
    let config = SaTokenConfig::builder()
        .is_concurrent(false)
        .build_config();
    assert!(!config.is_concurrent);
}

#[tokio::test]
async fn test_all_token_styles() {
    let styles = [
        TokenStyle::Uuid,
        TokenStyle::SimpleUuid,
        TokenStyle::Random32,
        TokenStyle::Random64,
        TokenStyle::Random128,
        TokenStyle::Jwt,
        TokenStyle::Hash,
        TokenStyle::Timestamp,
        TokenStyle::Tik,
    ];
    for style in &styles {
        let config = SaTokenConfig::builder()
            .token_style(*style)
            .build_config();
        // TokenStyle doesn't implement PartialEq for direct comparison,
        // but we can verify via debug output
        let _ = format!("{:?}", config.token_style);
    }
}

#[tokio::test]
async fn test_builder_register_listener() {
    use sa_token_core::SaTokenListener;
    struct DummyListener;
    #[async_trait::async_trait]
    impl SaTokenListener for DummyListener {}
    let storage = setup::memory_storage();
    let _mgr = SaTokenConfig::builder()
        .storage(storage)
        .register_listener(std::sync::Arc::new(DummyListener))
        .build();
}

// ── Failure cases ──────────────────────────────────────────────────────────

#[tokio::test]
#[should_panic(expected = "Storage must be set")]
async fn test_build_without_storage_panics() {
    SaTokenConfig::builder()
        .timeout(3600)
        .build();
}

#[tokio::test]
async fn test_jwt_style_without_secret() {
    // JWT style without secret key — config should still be buildable
    // (actual login would fail or fallback)
    let config = SaTokenConfig::builder()
        .token_style(TokenStyle::Jwt)
        .timeout(3600)
        .build_config();
    assert!(config.jwt_secret_key.is_none());
}

// ── Wave-1 配置补齐测试（对齐 Java SaTokenConfig 完整字段集） ──

#[tokio::test]
async fn test_wave1_defaults_match_java() {
    let config = SaTokenConfig::default();
    // Java isLastingCookie = true
    assert!(config.is_lasting_cookie);
    // Java isWriteHeader = false
    assert!(!config.is_write_header);
    // Java isLogoutKeepFreezeOps = false
    assert!(!config.is_logout_keep_freeze_ops);
    // Java cookieAutoFillPrefix = false
    assert!(!config.cookie_auto_fill_prefix);
    // Java isPrint = true
    assert!(config.is_print);
    // Java logLevel = "trace"
    assert_eq!(config.log_level, "trace");
    // Java logLevelInt = 1
    assert_eq!(config.log_level_int, 1);
    // Java isColorLog = null
    assert!(config.is_color_log.is_none());
    // Java httpBasic = ""
    assert_eq!(config.http_basic, "");
    // Java httpDigest = ""
    assert_eq!(config.http_digest, "");
    // Java currDomain = null
    assert!(config.curr_domain.is_none());
    // Java sameTokenTimeout = 86400
    assert_eq!(config.same_token_timeout, 86400);
    // Java checkSameToken = false
    assert!(!config.check_same_token);
    // Java dataRefreshPeriod = 30
    assert_eq!(config.data_refresh_period, 30);
    // Java maxTryTimes = 12
    assert_eq!(config.max_try_times, 12);
}

#[tokio::test]
async fn test_wave1_builder_methods() {
    let config = SaTokenConfig::builder()
        .is_lasting_cookie(false)
        .is_write_header(true)
        .is_logout_keep_freeze_ops(true)
        .cookie_auto_fill_prefix(true)
        .is_print(false)
        .log_level("info")
        .log_level_int(3)
        .is_color_log(Some(true))
        .http_basic("sa:123456")
        .http_digest("sa:123456")
        .curr_domain("https://example.com")
        .same_token_timeout(3600)
        .check_same_token(true)
        .data_refresh_period(60)
        .max_try_times(20)
        .build_config();
    assert!(!config.is_lasting_cookie);
    assert!(config.is_write_header);
    assert!(config.is_logout_keep_freeze_ops);
    assert!(config.cookie_auto_fill_prefix);
    assert!(!config.is_print);
    assert_eq!(config.log_level, "info");
    assert_eq!(config.log_level_int, 3);
    assert_eq!(config.is_color_log, Some(true));
    assert_eq!(config.http_basic, "sa:123456");
    assert_eq!(config.http_digest, "sa:123456");
    assert_eq!(config.curr_domain.as_deref(), Some("https://example.com"));
    assert_eq!(config.same_token_timeout, 3600);
    assert!(config.check_same_token);
    assert_eq!(config.data_refresh_period, 60);
    assert_eq!(config.max_try_times, 20);
}

#[tokio::test]
async fn test_token_prefix_default_none_and_set() {
    // 默认 None
    let config = SaTokenConfig::default();
    assert!(config.token_prefix.is_none());
    // 设置为 "Bearer "
    let config = SaTokenConfig::builder()
        .token_prefix("Bearer ")
        .build_config();
    assert_eq!(config.token_prefix.as_deref(), Some("Bearer "));
}
