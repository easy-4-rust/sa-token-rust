use sa_token_adapter::{CookieOptions, CookieSecurityPolicy};
use sa_token_core::config::TokenStyle;
use sa_token_core::token::TokenGenerator;
use sa_token_core::{ApiKeyTemplate, SaTokenConfig};
use sa_token_storage_memory::MemoryStorage;
use serde_json::Value;
use std::sync::Arc;

const GOLDEN: &str = include_str!("java_golden/sa_token_1_45_0.json");
const BASE62: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

fn golden() -> Value {
    serde_json::from_str(GOLDEN).expect("committed Java golden fixture must be valid JSON")
}

fn assert_base62(value: &str) {
    assert!(value.chars().all(|ch| BASE62.contains(ch)));
}

#[test]
fn java_source_is_explicitly_pinned() {
    let golden = golden();
    assert_eq!(golden["source"]["repository"], "Sa-Token");
    assert_eq!(golden["source"]["version"], "1.45.0");
    assert_eq!(
        golden["source"]["commit"],
        "902886c2149261ccb53a9c982068b7ccd0990237"
    );
}

#[test]
fn rust_defaults_match_pinned_java_contract() {
    let golden = golden();
    let expected = &golden["config_defaults"];
    let actual = SaTokenConfig::default();

    assert_eq!(actual.token_name, expected["token_name"]);
    assert_eq!(actual.timeout, expected["timeout"]);
    assert_eq!(actual.active_timeout, expected["active_timeout"]);
    assert_eq!(
        actual.dynamic_active_timeout,
        expected["dynamic_active_timeout"]
    );
    assert_eq!(actual.is_concurrent, expected["is_concurrent"]);
    assert_eq!(actual.is_share, expected["is_share"]);
    assert_eq!(actual.max_login_count, expected["max_login_count"]);
    assert_eq!(actual.max_try_times, expected["max_try_times"]);
    assert_eq!(actual.is_read_body, expected["is_read_body"]);
    assert_eq!(actual.is_read_header, expected["is_read_header"]);
    assert_eq!(actual.is_read_cookie, expected["is_read_cookie"]);
    assert_eq!(actual.is_lasting_cookie, expected["is_lasting_cookie"]);
    assert_eq!(actual.is_write_header, expected["is_write_header"]);
    assert!(matches!(actual.token_style, TokenStyle::Uuid));
    assert_eq!(actual.data_refresh_period, expected["data_refresh_period"]);
    assert_eq!(
        actual.token_session_check_login,
        expected["token_session_check_login"]
    );
    assert_eq!(actual.auto_renew, expected["auto_renew"]);
    assert_eq!(
        actual.cookie_auto_fill_prefix,
        expected["cookie_auto_fill_prefix"]
    );
    assert_eq!(actual.is_print, expected["is_print"]);
    assert_eq!(actual.is_log, expected["is_log"]);
}

#[test]
fn token_styles_match_java_observable_contracts() {
    let golden = golden();
    let styles = &golden["token_styles"];

    for _ in 0..256 {
        let uuid = TokenGenerator::generate_uuid().to_string();
        assert_eq!(
            uuid.len(),
            styles["uuid"]["length"].as_u64().unwrap() as usize
        );
        assert_eq!(
            uuid.chars().filter(|ch| *ch == '-').count(),
            styles["uuid"]["hyphens"].as_u64().unwrap() as usize
        );

        let simple_uuid = TokenGenerator::generate_simple_uuid().to_string();
        assert_eq!(
            simple_uuid.len(),
            styles["simple_uuid"]["length"].as_u64().unwrap() as usize
        );
        assert_base62(&simple_uuid);

        for (style, length) in [
            (TokenStyle::Random32, 32),
            (TokenStyle::Random64, 64),
            (TokenStyle::Random128, 128),
        ] {
            let config = SaTokenConfig {
                token_style: style,
                ..SaTokenConfig::default()
            };
            let token = TokenGenerator::generate(&config).unwrap().to_string();
            assert_eq!(token.len(), length);
            assert_base62(&token);
        }

        let config = SaTokenConfig {
            token_style: TokenStyle::Tik,
            ..SaTokenConfig::default()
        };
        let tik = TokenGenerator::generate(&config).unwrap().to_string();
        assert_eq!(
            tik.len(),
            styles["tik"]["length"].as_u64().unwrap() as usize
        );
        assert_eq!(tik.as_bytes()[2], b'_');
        assert_eq!(tik.as_bytes()[17], b'_');
        assert!(tik.ends_with("__"));
        assert_base62(&tik[0..2]);
        assert_base62(&tik[3..17]);
        assert_base62(&tik[18..34]);
    }
}

#[test]
fn java_cookie_compatibility_is_explicit_while_default_stays_secure() {
    let golden = golden();
    let java = &golden["java_cookie_defaults"];
    let compatibility = CookieOptions::for_policy(CookieSecurityPolicy::JavaCompatibility);

    assert_eq!(compatibility.domain.is_none(), java["domain_is_null"]);
    assert_eq!(compatibility.path.is_none(), java["path_is_null"]);
    assert_eq!(compatibility.secure, java["secure"]);
    assert_eq!(compatibility.http_only, java["http_only"]);
    assert_eq!(compatibility.same_site.is_none(), java["same_site_is_null"]);

    let production = CookieOptions::default();
    assert!(production.secure);
    assert!(production.http_only);
    assert!(production.same_site.is_some());
}

#[test]
fn api_key_defaults_match_java_contract() {
    let golden = golden();
    let expected = &golden["apikey_defaults"];
    let template = ApiKeyTemplate::new(Arc::new(MemoryStorage::new()), "sa:");
    let key = template.random_api_key_value().unwrap();

    assert_eq!(template.config().prefix, expected["prefix"]);
    assert_eq!(template.config().timeout, expected["timeout"]);
    assert_eq!(template.config().record_index, expected["record_index"]);
    assert!(key.starts_with(expected["prefix"].as_str().unwrap()));
    assert_eq!(
        key.len() - template.config().prefix.len(),
        expected["random_length"].as_u64().unwrap() as usize
    );
    assert_base62(&key[template.config().prefix.len()..]);
}
