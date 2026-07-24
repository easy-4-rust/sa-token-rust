// Author: 金书记
//
//! API Sign + API Key authentication module | API 签名 + API Key 鉴权模块
//!
//! 对应 Java:
//! - `cn.dev33.satoken.sign.template.SaSignTemplate`
//! - `cn.dev33.satoken.apikey.template.SaApiKeyTemplate`
//!
//! ## API Sign
//!
//! 基于参数签名 + 时间戳 + nonce 的防篡改防重放机制。
//!
//! ## API Key
//!
//! API Key 生命周期管理（创建/存储/校验/Scope/索引）。

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use sa_token_adapter::storage::{Expiration, SaStorage};
use uuid::Uuid;

use crate::constant_time::ct_eq_str;
use crate::error::SaTokenError;
use crate::token::TokenGenerator;

fn storage_err(e: sa_token_adapter::storage::StorageError) -> SaTokenError {
    SaTokenError::StorageError(e.to_string())
}

// ── API Sign ──

/// API Sign configuration
/// 对应 Java `SaSignConfig`
#[derive(Clone, Debug)]
pub struct SignConfig {
    /// API 调用签名密钥
    pub secret_key: String,
    /// 允许的时间戳差距（毫秒），-1 = 不校验，默认 15 分钟
    pub timestamp_disparity: i64,
}

impl Default for SignConfig {
    fn default() -> Self {
        Self {
            secret_key: String::new(),
            timestamp_disparity: 1000 * 60 * 15, // 15 min
        }
    }
}

/// API Sign template
/// 对应 Java `SaSignTemplate`
pub struct SignTemplate {
    config: SignConfig,
    storage: Arc<dyn SaStorage>,
    key_prefix: String,
}

impl SignTemplate {
    pub fn new(
        config: SignConfig,
        storage: Arc<dyn SaStorage>,
        key_prefix: impl Into<String>,
    ) -> Self {
        Self {
            config,
            storage,
            key_prefix: key_prefix.into(),
        }
    }

    /// Create a signature from a parameter map using HMAC-SHA256.
    ///
    /// 对应 Java `createSign(params)`（Java 用 MD5，Rust 升级为 HMAC-SHA256）：
    /// 1. Remove `sign` key
    /// 2. Sort params alphabetically
    /// 3. Build `k1=v1&k2=v2...`
    /// 4. HMAC-SHA256(params_str, secretKey)
    pub fn create_sign(&self, params: &BTreeMap<String, String>) -> String {
        let mut sorted = params.clone();
        sorted.remove("sign");

        let mut parts: Vec<String> = Vec::new();
        for (k, v) in &sorted {
            if !v.is_empty() {
                parts.push(format!("{k}={v}"));
            }
        }
        let full_str = parts.join("&");

        hmac_sha256_hex(&full_str, &self.config.secret_key)
    }

    /// Add timestamp, nonce, and sign to a parameter map.
    /// 对应 Java `addSignParams(params)`。
    pub fn add_sign_params(&self, params: &mut BTreeMap<String, String>) {
        let timestamp = now_millis().to_string();
        let nonce = Uuid::new_v4().simple().to_string();

        params.insert("timestamp".to_string(), timestamp);
        params.insert("nonce".to_string(), nonce);

        let sign = self.create_sign(params);
        params.insert("sign".to_string(), sign);
    }

    /// Check if timestamp is within allowed range.
    /// 对应 Java `isValidTimestamp(ts)`。
    pub fn is_valid_timestamp(&self, timestamp: i64) -> bool {
        if self.config.timestamp_disparity < 0 {
            return true;
        }
        let now = now_millis();
        (now - timestamp).abs() <= self.config.timestamp_disparity
    }

    /// Validate timestamp, returning error if invalid.
    pub fn check_timestamp(&self, timestamp: i64) -> Result<(), SaTokenError> {
        if !self.is_valid_timestamp(timestamp) {
            return Err(SaTokenError::FirewallCheck {
                message: "Timestamp validation failed: request expired".to_string(),
            });
        }
        Ok(())
    }

    /// Check if nonce has not been used (anti-replay).
    /// 对应 Java `isValidNonce(nonce)`。
    pub async fn is_valid_nonce(&self, nonce: &str) -> Result<bool, SaTokenError> {
        if nonce.is_empty() {
            return Ok(false);
        }
        let key = self.splicing_nonce_key(nonce);
        let exists = self.storage.exists(&key).await.map_err(storage_err)?;
        Ok(!exists)
    }

    /// Validate nonce and mark as used.
    /// 对应 Java `checkNonce(nonce)`。
    pub async fn check_nonce(&self, nonce: &str) -> Result<(), SaTokenError> {
        if nonce.is_empty() {
            return Err(SaTokenError::FirewallCheck {
                message: "Nonce is empty".to_string(),
            });
        }
        let key = self.splicing_nonce_key(nonce);
        let inserted = self
            .storage
            .set_if_absent(
                &key,
                "1",
                Expiration::After(std::time::Duration::from_secs(1800)),
            )
            .await
            .map_err(storage_err)?;
        if !inserted {
            return Err(SaTokenError::NonceAlreadyUsed);
        }
        Ok(())
    }

    /// Validate the sign parameter.
    /// 对应 Java `checkSign(params, providedSign)`。
    pub fn check_sign(
        &self,
        params: &BTreeMap<String, String>,
        provided_sign: &str,
    ) -> Result<(), SaTokenError> {
        let expected = self.create_sign(params);
        if ct_eq_str(&expected, provided_sign) {
            Ok(())
        } else {
            Err(SaTokenError::FirewallCheck {
                message: "Sign validation failed: signature mismatch".to_string(),
            })
        }
    }

    /// Full validation: timestamp + nonce + sign.
    /// 对应 Java `checkParamMap(params)`。
    pub async fn check_params(
        &self,
        params: &BTreeMap<String, String>,
    ) -> Result<(), SaTokenError> {
        let timestamp: i64 = params
            .get("timestamp")
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| SaTokenError::FirewallCheck {
                message: "Missing timestamp parameter".to_string(),
            })?;
        let nonce = params
            .get("nonce")
            .ok_or_else(|| SaTokenError::FirewallCheck {
                message: "Missing nonce parameter".to_string(),
            })?;
        let sign = params
            .get("sign")
            .ok_or_else(|| SaTokenError::FirewallCheck {
                message: "Missing sign parameter".to_string(),
            })?;

        self.check_timestamp(timestamp)?;
        self.check_sign(params, sign)?;
        self.check_nonce(nonce).await?;
        Ok(())
    }

    fn splicing_nonce_key(&self, nonce: &str) -> String {
        format!("{}sign:nonce:{}", self.key_prefix, nonce)
    }
}

// ── API Key ──

/// API Key model
/// 对应 Java `ApiKeyModel`
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ApiKeyModel {
    /// 名称
    pub title: String,
    /// 描述
    pub intro: String,
    /// API Key value
    pub api_key: String,
    /// 关联的 login_id
    pub login_id: String,
    /// 创建时间（毫秒）
    pub create_time: i64,
    /// 过期时间（毫秒），-1 = 永不过期
    pub expires_time: i64,
    /// 是否有效
    pub is_valid: bool,
    /// 授权范围
    pub scopes: Vec<String>,
    /// 扩展数据
    #[serde(default)]
    pub extra_data: BTreeMap<String, serde_json::Value>,
}

impl ApiKeyModel {
    /// Create a new API Key model with default values
    pub fn new(login_id: impl Into<String>) -> Self {
        Self {
            title: String::new(),
            intro: String::new(),
            api_key: String::new(),
            login_id: login_id.into(),
            create_time: now_millis(),
            expires_time: -1,
            is_valid: true,
            scopes: Vec::new(),
            extra_data: BTreeMap::new(),
        }
    }

    /// Check if this key has expired
    pub fn is_expired(&self) -> bool {
        if self.expires_time < 0 {
            return false;
        }
        now_millis() > self.expires_time
    }

    /// Check if this key has a specific scope
    pub fn has_scope(&self, scope: &str) -> bool {
        self.scopes.iter().any(|s| s == scope)
    }

    pub fn add_scope(&mut self, scope: impl Into<String>) -> &mut Self {
        self.scopes.push(scope.into());
        self
    }

    pub fn add_extra(&mut self, key: impl Into<String>, value: serde_json::Value) -> &mut Self {
        self.extra_data.insert(key.into(), value);
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApiKeyConfig {
    /// Java-compatible key prefix.
    pub prefix: String,
    /// Default lifetime in seconds; `-1` means permanent.
    pub timeout: i64,
    /// Maintain a race-free per-key reverse index.
    pub record_index: bool,
}

impl Default for ApiKeyConfig {
    fn default() -> Self {
        Self {
            prefix: "AK-".to_string(),
            timeout: 2_592_000,
            record_index: true,
        }
    }
}

/// API Key template
/// 对应 Java `SaApiKeyTemplate`
pub struct ApiKeyTemplate {
    storage: Arc<dyn SaStorage>,
    key_prefix: String,
    namespace: String,
    config: ApiKeyConfig,
}

impl ApiKeyTemplate {
    pub fn new(storage: Arc<dyn SaStorage>, key_prefix: impl Into<String>) -> Self {
        Self {
            storage,
            key_prefix: key_prefix.into(),
            namespace: "apikey".to_string(),
            config: ApiKeyConfig::default(),
        }
    }

    pub fn with_config(
        storage: Arc<dyn SaStorage>,
        key_prefix: impl Into<String>,
        config: ApiKeyConfig,
    ) -> Self {
        Self {
            storage,
            key_prefix: key_prefix.into(),
            namespace: "apikey".to_string(),
            config,
        }
    }

    pub fn config(&self) -> &ApiKeyConfig {
        &self.config
    }

    fn splicing_key(&self, api_key: &str) -> String {
        format!("{}{}:{}", self.key_prefix, self.namespace, api_key)
    }

    fn index_prefix(&self, login_id: &str) -> String {
        format!(
            "{}{}:index:{}:",
            self.key_prefix,
            self.namespace,
            hex::encode(login_id.as_bytes())
        )
    }

    fn index_key(&self, login_id: &str, api_key: &str) -> String {
        format!("{}{}", self.index_prefix(login_id), api_key)
    }

    /// Generate a random API Key value (Java: `prefix + 36-char random`)
    pub fn random_api_key_value(&self) -> Result<String, SaTokenError> {
        Ok(format!(
            "{}{}",
            self.config.prefix,
            TokenGenerator::generate_random(36)?.as_str()
        ))
    }

    /// Create and store a new API Key.
    /// 对应 Java `saveApiKey(model)`。
    pub async fn create_api_key(
        &self,
        login_id: impl Into<String>,
        timeout_seconds: i64,
    ) -> Result<ApiKeyModel, SaTokenError> {
        let mut model = ApiKeyModel::new(login_id);
        model.api_key = self.random_api_key_value()?;
        model.expires_time = if timeout_seconds == -1 {
            -1
        } else if timeout_seconds >= 0 {
            now_millis().saturating_add(timeout_seconds.saturating_mul(1000))
        } else {
            return Err(SaTokenError::ConfigError(
                "API Key timeout must be -1 or non-negative".to_string(),
            ));
        };
        self.save_api_key(&model, timeout_seconds).await?;
        Ok(model)
    }

    pub async fn create_api_key_default(
        &self,
        login_id: impl Into<String>,
    ) -> Result<ApiKeyModel, SaTokenError> {
        self.create_api_key(login_id, self.config.timeout).await
    }

    pub async fn save_api_key(
        &self,
        model: &ApiKeyModel,
        timeout_seconds: i64,
    ) -> Result<(), SaTokenError> {
        if model.api_key.is_empty() || model.login_id.is_empty() {
            return Err(SaTokenError::ConfigError(
                "API Key value and login_id must not be empty".to_string(),
            ));
        }
        let json = serde_json::to_string(&model).map_err(SaTokenError::SerializationError)?;
        let ttl = if timeout_seconds > 0 {
            Some(std::time::Duration::from_secs(timeout_seconds as u64))
        } else {
            None
        };
        self.storage
            .set(&self.splicing_key(&model.api_key), &json, ttl)
            .await
            .map_err(storage_err)?;
        if self.config.record_index {
            self.storage
                .set(&self.index_key(&model.login_id, &model.api_key), "", ttl)
                .await
                .map_err(storage_err)?;
        }
        Ok(())
    }

    /// Get an API Key model from storage.
    /// 对应 Java `getApiKeyModelFromCache(key)`。
    pub async fn get_api_key(&self, api_key: &str) -> Result<Option<ApiKeyModel>, SaTokenError> {
        let json = self
            .storage
            .get(&self.splicing_key(api_key))
            .await
            .map_err(storage_err)?;
        match json {
            Some(s) => {
                let model: ApiKeyModel =
                    serde_json::from_str(&s).map_err(SaTokenError::SerializationError)?;
                Ok(Some(model))
            }
            None => Ok(None),
        }
    }

    /// Validate an API Key (exists, not expired, enabled).
    /// 对应 Java `checkApiKey(key)`。
    pub async fn check_api_key(&self, api_key: &str) -> Result<ApiKeyModel, SaTokenError> {
        let model = self
            .get_api_key(api_key)
            .await?
            .ok_or(SaTokenError::TokenNotFound)?;
        if model.is_expired() {
            return Err(SaTokenError::TokenExpired);
        }
        if !model.is_valid {
            return Err(SaTokenError::PermissionDenied);
        }
        Ok(model)
    }

    /// Delete an API Key.
    pub async fn delete_api_key(&self, api_key: &str) -> Result<(), SaTokenError> {
        let model = self.get_api_key(api_key).await?;
        self.storage
            .delete(&self.splicing_key(api_key))
            .await
            .map_err(storage_err)?;
        if let Some(model) = model
            && self.config.record_index
        {
            self.storage
                .delete(&self.index_key(&model.login_id, api_key))
                .await
                .map_err(storage_err)?;
        }
        Ok(())
    }

    pub async fn get_api_keys_by_login_id(
        &self,
        login_id: &str,
    ) -> Result<Vec<ApiKeyModel>, SaTokenError> {
        if !self.config.record_index {
            return Err(SaTokenError::ApiDisabled);
        }
        let prefix = self.index_prefix(login_id);
        let pattern = format!("{prefix}*");
        let mut cursor = None;
        let mut models = Vec::new();
        loop {
            let page = self
                .storage
                .scan(&pattern, cursor.as_deref(), 128)
                .await
                .map_err(storage_err)?;
            for key in page.keys {
                if let Some(api_key) = key.strip_prefix(&prefix)
                    && let Some(model) = self.get_api_key(api_key).await?
                {
                    models.push(model);
                }
            }
            match page.next_cursor {
                Some(next) if cursor.as_deref() != Some(next.as_str()) => cursor = Some(next),
                _ => break,
            }
        }
        models.sort_by(|left, right| left.api_key.cmp(&right.api_key));
        Ok(models)
    }

    pub async fn delete_api_keys_by_login_id(&self, login_id: &str) -> Result<usize, SaTokenError> {
        let models = self.get_api_keys_by_login_id(login_id).await?;
        let count = models.len();
        for model in models {
            self.delete_api_key(&model.api_key).await?;
        }
        Ok(count)
    }

    /// Get login_id by API Key.
    pub async fn get_login_id_by_api_key(&self, api_key: &str) -> Result<String, SaTokenError> {
        let model = self.check_api_key(api_key).await?;
        Ok(model.login_id)
    }

    /// Check if API Key has all specified scopes (AND mode).
    pub async fn check_api_key_scope(
        &self,
        api_key: &str,
        required_scopes: &[&str],
    ) -> Result<(), SaTokenError> {
        let model = self.check_api_key(api_key).await?;
        for scope in required_scopes {
            if !model.has_scope(scope) {
                return Err(SaTokenError::PermissionDeniedDetail(format!(
                    "Missing API Key scope: {scope}"
                )));
            }
        }
        Ok(())
    }

    /// Check whether at least one required scope is present (OR mode).
    pub async fn check_api_key_scope_any(
        &self,
        api_key: &str,
        required_scopes: &[&str],
    ) -> Result<(), SaTokenError> {
        let model = self.check_api_key(api_key).await?;
        if required_scopes.is_empty() || required_scopes.iter().any(|scope| model.has_scope(scope))
        {
            return Ok(());
        }
        Err(SaTokenError::PermissionDeniedDetail(format!(
            "Missing any API Key scope from: {}",
            required_scopes.join(", ")
        )))
    }
}

// ── Utilities ──

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// HMAC-SHA256 hex digest (RFC 2104, for API Sign — more secure than MD5).
///
/// 手动实现而非引入 hmac crate，保持与 sa-token-rust 原有依赖管理一致。
fn hmac_sha256_hex(data: &str, key: &str) -> String {
    use sha2::{Digest, Sha256};

    const BLOCK_SIZE: usize = 64; // SHA-256 block size

    // Normalize key: if longer than block size, hash it
    let key_bytes = if key.len() > BLOCK_SIZE {
        let mut h = Sha256::new();
        h.update(key.as_bytes());
        h.finalize().to_vec()
    } else {
        key.as_bytes().to_vec()
    };

    // Pad key to block size
    let mut padded_key = [0u8; BLOCK_SIZE];
    padded_key[..key_bytes.len()].copy_from_slice(&key_bytes);

    // Create ipad and opad
    let mut ipad = [0u8; BLOCK_SIZE];
    let mut opad = [0u8; BLOCK_SIZE];
    for i in 0..BLOCK_SIZE {
        ipad[i] = padded_key[i] ^ 0x36;
        opad[i] = padded_key[i] ^ 0x5c;
    }

    // Inner hash: H(ipad || data)
    let mut inner = Sha256::new();
    inner.update(ipad);
    inner.update(data.as_bytes());
    let inner_result = inner.finalize();

    // Outer hash: H(opad || inner_hash)
    let mut outer = Sha256::new();
    outer.update(opad);
    outer.update(inner_result);
    let result = outer.finalize();

    result.iter().map(|b| format!("{:02x}", b)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sa_token_storage_memory::MemoryStorage;

    fn make_storage() -> Arc<dyn SaStorage> {
        Arc::new(MemoryStorage::new())
    }

    // ── Sign tests ──

    #[test]
    fn test_sign_create_sign() {
        let cfg = SignConfig {
            secret_key: "my-secret".to_string(),
            timestamp_disparity: -1,
        };
        let tpl = SignTemplate::new(cfg, make_storage(), "sa:");
        let mut params = BTreeMap::new();
        params.insert("name".to_string(), "value".to_string());
        params.insert("id".to_string(), "123".to_string());
        let sign = tpl.create_sign(&params);
        assert!(!sign.is_empty());
        assert_eq!(sign.len(), 64); // HMAC-SHA256 hex = 64 chars
    }

    #[test]
    fn test_sign_check_success() {
        let cfg = SignConfig {
            secret_key: "secret".to_string(),
            timestamp_disparity: -1,
        };
        let tpl = SignTemplate::new(cfg, make_storage(), "sa:");
        let mut params = BTreeMap::new();
        params.insert("key".to_string(), "value".to_string());
        let sign = tpl.create_sign(&params);
        assert!(tpl.check_sign(&params, &sign).is_ok());
    }

    #[test]
    fn test_sign_check_failure() {
        let cfg = SignConfig {
            secret_key: "secret".to_string(),
            timestamp_disparity: -1,
        };
        let tpl = SignTemplate::new(cfg, make_storage(), "sa:");
        let params = BTreeMap::new();
        assert!(tpl.check_sign(&params, "wrong-sign").is_err());
    }

    #[test]
    fn test_sign_timestamp_valid() {
        let cfg = SignConfig {
            secret_key: "k".to_string(),
            timestamp_disparity: 60_000, // 1 min
        };
        let tpl = SignTemplate::new(cfg, make_storage(), "sa:");
        assert!(tpl.is_valid_timestamp(now_millis()));
        assert!(tpl.is_valid_timestamp(now_millis() - 30_000));
        assert!(!tpl.is_valid_timestamp(now_millis() - 120_000));
    }

    #[test]
    fn test_sign_add_sign_params() {
        let cfg = SignConfig::default();
        let tpl = SignTemplate::new(cfg, make_storage(), "sa:");
        let mut params = BTreeMap::new();
        params.insert("data".to_string(), "hello".to_string());
        tpl.add_sign_params(&mut params);
        assert!(params.contains_key("timestamp"));
        assert!(params.contains_key("nonce"));
        assert!(params.contains_key("sign"));
    }

    #[tokio::test]
    async fn test_sign_nonce_anti_replay() {
        let cfg = SignConfig::default();
        let tpl = SignTemplate::new(cfg, make_storage(), "sa:");
        tpl.check_nonce("nonce-1").await.unwrap();
        // Second use should fail (replay)
        assert!(tpl.check_nonce("nonce-1").await.is_err());
    }

    #[tokio::test]
    async fn invalid_signature_does_not_consume_nonce() {
        let tpl = SignTemplate::new(SignConfig::default(), make_storage(), "sa:");
        let mut params = BTreeMap::new();
        params.insert("data".to_string(), "payload".to_string());
        tpl.add_sign_params(&mut params);
        params.insert("sign".to_string(), "invalid".to_string());

        assert!(tpl.check_params(&params).await.is_err());

        let valid_sign = tpl.create_sign(&params);
        params.insert("sign".to_string(), valid_sign);
        assert!(tpl.check_params(&params).await.is_ok());
    }

    #[tokio::test]
    async fn concurrent_signed_request_consumes_nonce_exactly_once() {
        let tpl = Arc::new(SignTemplate::new(
            SignConfig::default(),
            make_storage(),
            "sa:",
        ));
        let mut params = BTreeMap::new();
        params.insert("data".to_string(), "payload".to_string());
        tpl.add_sign_params(&mut params);
        let params = Arc::new(params);

        let mut tasks = Vec::new();
        for _ in 0..32 {
            let tpl = tpl.clone();
            let params = params.clone();
            tasks.push(tokio::spawn(async move { tpl.check_params(&params).await }));
        }

        let mut accepted = 0;
        let mut rejected_as_replay = 0;
        for task in tasks {
            match task.await.unwrap() {
                Ok(()) => accepted += 1,
                Err(SaTokenError::NonceAlreadyUsed) => rejected_as_replay += 1,
                Err(error) => panic!("unexpected validation error: {error}"),
            }
        }
        assert_eq!(accepted, 1);
        assert_eq!(rejected_as_replay, 31);
    }

    // ── API Key tests ──

    #[tokio::test]
    async fn test_apikey_create_and_get() {
        let tpl = ApiKeyTemplate::new(make_storage(), "sa:");
        let model = tpl.create_api_key("user-1", 3600).await.unwrap();
        assert!(model.api_key.starts_with("AK-"));
        assert_eq!(model.api_key.len(), 39);
        let got = tpl.get_api_key(&model.api_key).await.unwrap();
        assert!(got.is_some());
        assert_eq!(got.unwrap().login_id, "user-1");
    }

    #[tokio::test]
    async fn test_apikey_check_valid() {
        let tpl = ApiKeyTemplate::new(make_storage(), "sa:");
        let model = tpl.create_api_key("user-1", 3600).await.unwrap();
        let checked = tpl.check_api_key(&model.api_key).await.unwrap();
        assert_eq!(checked.login_id, "user-1");
    }

    #[tokio::test]
    async fn test_apikey_check_nonexistent() {
        let tpl = ApiKeyTemplate::new(make_storage(), "sa:");
        assert!(tpl.check_api_key("sk-nonexistent").await.is_err());
    }

    #[tokio::test]
    async fn test_apikey_delete() {
        let tpl = ApiKeyTemplate::new(make_storage(), "sa:");
        let model = tpl.create_api_key("user-1", 3600).await.unwrap();
        tpl.delete_api_key(&model.api_key).await.unwrap();
        assert!(tpl.get_api_key(&model.api_key).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_apikey_get_login_id() {
        let tpl = ApiKeyTemplate::new(make_storage(), "sa:");
        let model = tpl.create_api_key("user-42", 3600).await.unwrap();
        let login_id = tpl.get_login_id_by_api_key(&model.api_key).await.unwrap();
        assert_eq!(login_id, "user-42");
    }

    #[tokio::test]
    async fn apikey_reverse_index_and_scope_modes() {
        let tpl = ApiKeyTemplate::new(make_storage(), "sa:");
        let mut first = tpl.create_api_key("user-1", 3600).await.unwrap();
        first
            .add_scope("read")
            .add_extra("tenant", serde_json::json!(7));
        tpl.save_api_key(&first, 3600).await.unwrap();

        let mut second = tpl.create_api_key("user-1", 3600).await.unwrap();
        second.add_scope("write");
        tpl.save_api_key(&second, 3600).await.unwrap();

        let indexed = tpl.get_api_keys_by_login_id("user-1").await.unwrap();
        assert_eq!(indexed.len(), 2);
        let indexed_first = indexed
            .iter()
            .find(|model| model.api_key == first.api_key)
            .unwrap();
        assert_eq!(
            indexed_first.extra_data.get("tenant"),
            Some(&serde_json::json!(7))
        );
        assert!(
            tpl.check_api_key_scope_any(&first.api_key, &["write", "read"])
                .await
                .is_ok()
        );
        assert!(
            tpl.check_api_key_scope(&first.api_key, &["read", "write"])
                .await
                .is_err()
        );

        assert_eq!(tpl.delete_api_keys_by_login_id("user-1").await.unwrap(), 2);
        assert!(tpl.get_api_key(&first.api_key).await.unwrap().is_none());
        assert!(tpl.get_api_key(&second.api_key).await.unwrap().is_none());
    }

    #[test]
    fn test_apikey_model_is_expired() {
        let mut model = ApiKeyModel::new("user-1");
        model.expires_time = -1;
        assert!(!model.is_expired());
        model.expires_time = now_millis() - 1000;
        assert!(model.is_expired());
    }

    #[test]
    fn test_apikey_model_has_scope() {
        let mut model = ApiKeyModel::new("user-1");
        model.scopes = vec!["read".to_string(), "write".to_string()];
        assert!(model.has_scope("read"));
        assert!(!model.has_scope("admin"));
    }
}
