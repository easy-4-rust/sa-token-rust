//! Vernal Environment 到 Sa-Token 原生配置的真实绑定合同测试。

use std::sync::Arc;

use sa_token_core::{
    LogoutMode, ReplacedLoginExitMode, ReplacedRange, SaTokenConfig, config::TokenStyle,
};
use sa_token_vernal::{VernalSaTokenConfigBinder, VernalSaTokenConfigError};
use vernal_context::{ApplicationEnvironment, MapPropertySource};

fn environment(
    values: impl IntoIterator<Item = (&'static str, &'static str)>,
) -> ApplicationEnvironment {
    let mut builder = ApplicationEnvironment::builder();
    builder
        .add_last(Arc::new(
            MapPropertySource::new("test", values).expect("test property source"),
        ))
        .expect("environment source");
    builder.build()
}

#[test]
fn environment_overrides_supported_fields_and_preserves_native_defaults() {
    let environment = environment([
        ("security.token-name", "vernal-token"),
        ("security.timeout", "7200"),
        ("security.active-timeout", "600"),
        ("security.auto-renew", "false"),
        ("security.is-concurrent", "false"),
        ("security.is-share", "true"),
        ("security.token-style", "random_64"),
        ("security.token-prefix", "Bearer "),
        ("security.storage-key-prefix", "vernal:"),
        ("security.jwt-secret-key", "top-secret"),
        ("security.max-login-count", "3"),
        ("security.overflow-logout-mode", "kick-out"),
        ("security.replaced-login-exit-mode", "new-device"),
        ("security.replaced-range", "all-device-type"),
        ("security.is-write-header", "true"),
        ("security.same-token-timeout", "300"),
        ("security.data-refresh-period", "15"),
    ]);

    let config = VernalSaTokenConfigBinder::with_prefix("security")
        .expect("custom prefix")
        .build_config(&environment)
        .expect("Sa-Token config");

    assert_eq!(config.token_name, "vernal-token");
    assert_eq!(config.timeout, 7200);
    assert_eq!(config.active_timeout, 600);
    assert!(!config.auto_renew);
    assert!(!config.is_concurrent);
    assert!(config.is_share);
    assert!(matches!(config.token_style, TokenStyle::Random64));
    assert_eq!(config.token_prefix.as_deref(), Some("Bearer "));
    assert_eq!(config.storage_key_prefix, "vernal:");
    assert_eq!(config.jwt_secret_key.as_deref(), Some("top-secret"));
    assert_eq!(config.max_login_count, 3);
    assert_eq!(config.overflow_logout_mode, LogoutMode::KickOut);
    assert_eq!(
        config.replaced_login_exit_mode,
        ReplacedLoginExitMode::NewDevice
    );
    assert_eq!(config.replaced_range, ReplacedRange::AllDeviceType);
    assert!(config.is_write_header);
    assert_eq!(config.same_token_timeout, 300);
    assert_eq!(config.data_refresh_period, 15);

    // 没有声明的字段继续使用 Sa-Token 自己的默认值，而不是由桥接器复制默认值。
    assert_eq!(
        config.enable_refresh_token,
        SaTokenConfig::default().enable_refresh_token
    );
}

#[test]
fn invalid_enum_and_typed_values_do_not_leak_sensitive_property_values() {
    let invalid_enum = environment([
        ("sa-token.token-style", "secret-invalid-style"),
        ("sa-token.jwt-secret-key", "must-not-leak"),
    ]);
    let error = VernalSaTokenConfigBinder::new()
        .build_config(&invalid_enum)
        .expect_err("invalid token style");
    assert!(matches!(
        error,
        VernalSaTokenConfigError::InvalidChoice { .. }
    ));
    let rendered = format!("{error:?} {error}");
    assert!(!rendered.contains("secret-invalid-style"));
    assert!(!rendered.contains("must-not-leak"));

    let invalid_typed = environment([
        ("sa-token.timeout", "private-non-number"),
        ("sa-token.http-basic", "admin:private-password"),
    ]);
    let error = VernalSaTokenConfigBinder::new()
        .build_config(&invalid_typed)
        .expect_err("invalid timeout");
    let rendered = format!("{error:?} {error}");
    assert!(!rendered.contains("private-non-number"));
    assert!(!rendered.contains("admin:private-password"));
}
