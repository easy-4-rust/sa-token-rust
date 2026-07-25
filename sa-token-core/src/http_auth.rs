// Author: 金书记
//
//! HTTP Basic / Digest authentication module | HTTP Basic / Digest 认证模块
//!
//! 对应 Java `cn.dev33.satoken.httpauth.basic.SaHttpBasicTemplate`
//! 和 `cn.dev33.satoken.httpauth.digest.SaHttpDigestTemplate`。
//!
//! ## Design
//!
//! Java splits Basic and Digest into separate packages (6 files).
//! Rust consolidates into a single module (~350 lines) since both share
//! the same API pattern (check / get_authorization / throw_error).

use base64::Engine;
use md5::{Digest as Md5Digest, Md5};

use crate::constant_time::ct_eq_str;
use crate::error::SaTokenError;

/// Default realm for HTTP Basic/Digest challenges
/// 对应 Java `DEFAULT_REALM = "Sa-Token"`
pub const DEFAULT_REALM: &str = "sa-token";

/// Default QOP for Digest authentication
pub const DEFAULT_QOP: &str = "auth";

// ── HTTP Basic Authentication ──

/// HTTP Basic account (username:password)
/// 对应 Java `SaHttpBasicAccount`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpBasicAccount {
    pub username: String,
    pub password: String,
}

impl HttpBasicAccount {
    /// Create from explicit username and password
    pub fn new(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
        }
    }

    /// Parse from `"username:password"` format
    /// 对应 Java `SaHttpBasicAccount(String usernameAndPassword)`
    pub fn parse(encoded: &str) -> Result<Self, SaTokenError> {
        if encoded.is_empty() {
            return Err(SaTokenError::ConfigError(
                "UsernameAndPassword cannot be empty".to_string(),
            ));
        }
        let parts: Vec<&str> = encoded.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(SaTokenError::ConfigError(
                "UsernameAndPassword format error, expected: username:password".to_string(),
            ));
        }
        Ok(Self {
            username: parts[0].to_string(),
            password: parts[1].to_string(),
        })
    }

    /// Serialize back to `"username:password"`
    pub fn encode(&self) -> String {
        format!("{}:{}", self.username, self.password)
    }
}

impl std::fmt::Display for HttpBasicAccount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.username, self.password)
    }
}

/// HTTP Basic authentication utilities
/// 对应 Java `SaHttpBasicTemplate`
pub struct HttpBasicTemplate;

impl HttpBasicTemplate {
    /// Extract the decoded `"username:password"` from the `Authorization` header.
    ///
    /// Returns `None` if the header is missing or doesn't start with `"Basic "`.
    pub fn get_authorization_value(auth_header: Option<&str>) -> Option<String> {
        let header = auth_header?.trim();
        // Case-insensitive prefix match: "Basic " or "basic "
        if header.len() < 6 {
            return None;
        }
        let prefix = &header[..6];
        if !prefix.eq_ignore_ascii_case("Basic ") {
            return None;
        }
        let encoded = &header[6..].trim();
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .ok()?;
        String::from_utf8(decoded).ok()
    }

    /// Check Basic auth against a configured `"username:password"` string.
    ///
    /// 对应 Java `SaHttpBasicTemplate.check(realm, account)`。
    ///
    /// # Errors
    ///
    /// Returns `SaTokenError::NotHttpBasicAuth` if authentication fails.
    pub fn check(auth_header: Option<&str>, expected_account: &str) -> Result<(), SaTokenError> {
        let provided = Self::get_authorization_value(auth_header);
        match provided {
            Some(value) if ct_eq_str(&value, expected_account) => Ok(()),
            _ => Err(SaTokenError::NotHttpBasicAuth),
        }
    }

    /// Check Basic auth with explicit username and password.
    pub fn check_with_credentials(
        auth_header: Option<&str>,
        username: &str,
        password: &str,
    ) -> Result<(), SaTokenError> {
        let expected = format!("{username}:{password}");
        Self::check(auth_header, &expected)
    }

    /// Parse the Authorization header into an `HttpBasicAccount`.
    pub fn get_account(auth_header: Option<&str>) -> Option<HttpBasicAccount> {
        let value = Self::get_authorization_value(auth_header)?;
        HttpBasicAccount::parse(&value).ok()
    }
}

// ── HTTP Digest Authentication ──

/// Digest authentication model (all fields from the Digest spec)
/// 对应 Java `SaHttpDigestModel`
#[derive(Debug, Clone, Default)]
pub struct HttpDigestModel {
    pub username: String,
    pub password: String,
    pub realm: String,
    pub nonce: String,
    pub uri: String,
    pub method: String,
    pub qop: String,
    pub nc: String,
    pub cnonce: String,
    pub opaque: String,
    pub response: String,
}

impl HttpDigestModel {
    pub fn new(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
            realm: DEFAULT_REALM.to_string(),
            qop: DEFAULT_QOP.to_string(),
            ..Default::default()
        }
    }

    pub fn with_realm(mut self, realm: impl Into<String>) -> Self {
        self.realm = realm.into();
        self
    }
}

/// HTTP Digest authentication utilities
/// 对应 Java `SaHttpDigestTemplate`
pub struct HttpDigestTemplate;

impl HttpDigestTemplate {
    /// Extract the raw Digest value from the `Authorization` header.
    ///
    /// Returns `None` if missing or not Digest-prefixed.
    pub fn get_authorization_value(auth_header: Option<&str>) -> Option<String> {
        let header = auth_header?.trim();
        if header.len() < 7 {
            return None;
        }
        let prefix = &header[..7];
        if !prefix.eq_ignore_ascii_case("Digest ") {
            return None;
        }
        Some(header[7..].to_string())
    }

    /// Parse a raw Digest header value into a model.
    ///
    /// 对应 Java `getAuthorizationValueToModel()`。
    pub fn parse_digest(raw: &str) -> HttpDigestModel {
        let mut model = HttpDigestModel::default();
        // Split by comma, then by first "="
        for part in raw.split(',') {
            let part = part.trim();
            if let Some(eq_pos) = part.find('=') {
                let key = part[..eq_pos].trim();
                let val = part[eq_pos + 1..].trim().trim_matches('"');
                match key {
                    "username" => model.username = val.to_string(),
                    "realm" => model.realm = val.to_string(),
                    "nonce" => model.nonce = val.to_string(),
                    "uri" => model.uri = val.to_string(),
                    "qop" => model.qop = val.to_string(),
                    "nc" => model.nc = val.to_string(),
                    "cnonce" => model.cnonce = val.to_string(),
                    "opaque" => model.opaque = val.to_string(),
                    "response" => model.response = val.to_string(),
                    _ => {}
                }
            }
        }
        model
    }

    /// Calculate the expected Digest `response` value.
    ///
    /// 对应 Java `calcResponse(model)`：
    /// - `frag1 = md5(username:realm:password)`
    /// - `frag2 = nonce:nc:cnonce:qop`
    /// - `frag3 = md5(method:uri)`
    /// - `response = md5(frag1:frag2:frag3)`
    pub fn calc_response(model: &HttpDigestModel) -> String {
        let frag1 = md5_hex(&format!(
            "{}:{}:{}",
            model.username, model.realm, model.password
        ));
        let frag2 = format!(
            "{}:{}:{}:{}",
            model.nonce, model.nc, model.cnonce, model.qop
        );
        let frag3 = md5_hex(&format!("{}:{}", model.method, model.uri));
        md5_hex(&format!("{frag1}:{frag2}:{frag3}"))
    }

    /// Check Digest authentication.
    ///
    /// 对应 Java `SaHttpDigestTemplate.check(hopeModel)`。
    ///
    /// # Errors
    ///
    /// Returns `SaTokenError::NotHttpDigestAuth` if authentication fails.
    pub fn check(
        auth_header: Option<&str>,
        method: &str,
        username: &str,
        password: &str,
    ) -> Result<(), SaTokenError> {
        let raw =
            Self::get_authorization_value(auth_header).ok_or(SaTokenError::NotHttpDigestAuth)?;

        let mut req_model = Self::parse_digest(&raw);
        // Fill in server-known values
        req_model.username = username.to_string();
        req_model.password = password.to_string();
        req_model.method = method.to_string();

        if req_model.nonce.is_empty() || req_model.uri.is_empty() {
            return Err(SaTokenError::NotHttpDigestAuth);
        }

        let calculated = Self::calc_response(&req_model);
        if ct_eq_str(&calculated, &req_model.response) {
            Ok(())
        } else {
            Err(SaTokenError::NotHttpDigestAuth)
        }
    }

    /// Build a `WWW-Authenticate: Digest ...` header value for 401 responses.
    pub fn build_challenge(realm: &str, nonce: &str, opaque: &str) -> String {
        format!(
            r#"Digest realm="{}", qop="auth", nonce="{}", opaque="{}"#,
            realm, nonce, opaque
        )
    }
}

/// Compute MD5 hex digest of a string
fn md5_hex(input: &str) -> String {
    let mut hasher = Md5::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    result.iter().map(|b| format!("{:02x}", b)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_account_parse() {
        let acc = HttpBasicAccount::parse("admin:secret").unwrap();
        assert_eq!(acc.username, "admin");
        assert_eq!(acc.password, "secret");
    }

    #[test]
    fn test_basic_account_parse_empty_errors() {
        assert!(HttpBasicAccount::parse("").is_err());
    }

    #[test]
    fn test_basic_account_parse_no_colon_errors() {
        assert!(HttpBasicAccount::parse("nocolon").is_err());
    }

    #[test]
    fn test_basic_check_success() {
        // Base64 of "admin:secret" = "YWRtaW46c2VjcmV0"
        let header = "Basic YWRtaW46c2VjcmV0";
        assert!(HttpBasicTemplate::check(Some(header), "admin:secret").is_ok());
    }

    #[test]
    fn test_basic_check_wrong_password() {
        let header = "Basic YWRtaW46d3Jvbmc=";
        assert!(HttpBasicTemplate::check(Some(header), "admin:secret").is_err());
    }

    #[test]
    fn test_basic_check_missing_header() {
        assert!(HttpBasicTemplate::check(None, "admin:secret").is_err());
    }

    #[test]
    fn test_basic_check_not_basic_scheme() {
        assert!(HttpBasicTemplate::check(Some("Bearer xyz"), "admin:secret").is_err());
    }

    #[test]
    fn test_basic_get_account() {
        let header = "Basic YWRtaW46c2VjcmV0";
        let acc = HttpBasicTemplate::get_account(Some(header)).unwrap();
        assert_eq!(acc.username, "admin");
        assert_eq!(acc.password, "secret");
    }

    #[test]
    fn test_digest_calc_response() {
        let model = HttpDigestModel {
            username: "Mufasa".to_string(),
            password: "Circle Of Life".to_string(),
            realm: "testrealm@host.com".to_string(),
            nonce: "dcd98b7102dd2f0e8b11d0f600bfb0c093".to_string(),
            uri: "/dir/index.html".to_string(),
            method: "GET".to_string(),
            qop: "auth".to_string(),
            nc: "00000001".to_string(),
            cnonce: "0a4f113b".to_string(),
            ..Default::default()
        };
        let response = HttpDigestTemplate::calc_response(&model);
        // RFC 2617 example: expected response = "6629fae49393a05397450978507c4ef1"
        assert_eq!(response, "6629fae49393a05397450978507c4ef1");
    }

    #[test]
    fn test_digest_parse() {
        let raw = r#"username="admin", realm="sa-token", nonce="abc123", uri="/api", qop="auth", nc=00000001, cnonce="xyz", response="deadbeef""#;
        let model = HttpDigestTemplate::parse_digest(raw);
        assert_eq!(model.username, "admin");
        assert_eq!(model.realm, "sa-token");
        assert_eq!(model.nonce, "abc123");
        assert_eq!(model.uri, "/api");
        assert_eq!(model.response, "deadbeef");
    }

    #[test]
    fn test_digest_check_missing_header() {
        assert!(HttpDigestTemplate::check(None, "GET", "admin", "pass").is_err());
    }

    #[test]
    fn test_md5_hex() {
        assert_eq!(md5_hex(""), "d41d8cd98f00b204e9800998ecf8427e");
        assert_eq!(md5_hex("hello"), "5d41402abc4b2a76b9719d911017c592");
    }

    #[test]
    fn test_digest_build_challenge() {
        let challenge = HttpDigestTemplate::build_challenge("test", "nonce123", "opaque456");
        assert!(challenge.contains(r#"realm="test""#));
        assert!(challenge.contains(r#"nonce="nonce123""#));
        assert!(challenge.contains(r#"opaque="opaque456"#));
    }
}
