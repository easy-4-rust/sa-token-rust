// Author: 金书记
//
//! Same-Token and Temp-Token modules | 同源认证 + 临时 Token 模块
//!
//! 对应 Java:
//! - `cn.dev33.satoken.same.SaSameTemplate`
//! - `cn.dev33.satoken.temp.SaTempTemplate`

use std::sync::Arc;
use std::time::Duration;

use sa_token_adapter::storage::SaStorage;
use uuid::Uuid;

use crate::constant_time::ct_eq_str;
use crate::error::SaTokenError;

// ── Same-Token ──

/// Same-Token HTTP header name (Java `SAME_TOKEN = "SA-SAME-TOKEN"`)
pub const SAME_TOKEN_HEADER: &str = "SA-SAME-TOKEN";

/// Default token length for generated same-tokens
#[allow(dead_code)]
const SAME_TOKEN_LENGTH: usize = 64;

/// Same-Token template for same-origin RPC authentication.
///
/// 对应 Java `SaSameTemplate`。
/// Uses the storage's key prefix (default `sa:`) + `var:same-token`.
#[derive(Clone)]
pub struct SameTokenTemplate {
    storage: Arc<dyn SaStorage>,
    key_prefix: String,
    timeout_seconds: i64,
}

impl SameTokenTemplate {
    /// Create a new SameTokenTemplate.
    pub fn new(storage: Arc<dyn SaStorage>, key_prefix: impl Into<String>, timeout_seconds: i64) -> Self {
        Self {
            storage,
            key_prefix: key_prefix.into(),
            timeout_seconds,
        }
    }

    fn current_key(&self) -> String {
        format!("{}var:same-token", self.key_prefix)
    }

    fn past_key(&self) -> String {
        format!("{}var:past-same-token", self.key_prefix)
    }

    fn ttl_duration(&self) -> Option<Duration> {
        if self.timeout_seconds > 0 {
            Some(Duration::from_secs(self.timeout_seconds as u64))
        } else {
            None
        }
    }

    /// Generate a random 64-char token (Java `SaFoxUtil.getRandomString(64)`)
    fn create_token_value() -> String {
        // Use two UUIDs concatenated (32+32=64 chars) to avoid extra `rand` dependency
        let part1 = Uuid::new_v4().simple().to_string();
        let part2 = Uuid::new_v4().simple().to_string();
        format!("{part1}{part2}")
    }

    /// Get the current same-token, creating one if absent.
    pub async fn get_token(&self) -> Result<String, SaTokenError> {
        if let Some(token) = self.get_token_nh().await? {
            return Ok(token);
        }
        self.refresh_token().await
    }

    /// Get the current same-token without creating.
    pub async fn get_token_nh(&self) -> Result<Option<String>, SaTokenError> {
        self.storage
            .get(&self.current_key())
            .await
            .map_err(storage_err)
    }

    /// Get the past same-token.
    pub async fn get_past_token_nh(&self) -> Result<Option<String>, SaTokenError> {
        self.storage
            .get(&self.past_key())
            .await
            .map_err(storage_err)
    }

    /// Check if a token is valid (matches current or past).
    pub async fn is_valid(&self, token: &str) -> Result<bool, SaTokenError> {
        if token.is_empty() {
            return Ok(false);
        }
        let current = self.get_token_nh().await?.unwrap_or_default();
        if ct_eq_str(token, &current) {
            return Ok(true);
        }
        let past = self.get_past_token_nh().await?.unwrap_or_default();
        Ok(ct_eq_str(token, &past))
    }

    /// Validate a token, returning error if invalid.
    pub async fn check_token(&self, token: &str) -> Result<(), SaTokenError> {
        if !self.is_valid(token).await? {
            return Err(SaTokenError::SameTokenInvalid(
                "Same-Token is invalid or expired".to_string(),
            ));
        }
        Ok(())
    }

    /// Rotate the token: move current → past, generate new current.
    pub async fn refresh_token(&self) -> Result<String, SaTokenError> {
        if let Some(old) = self.get_token_nh().await? {
            self.storage
                .set(&self.past_key(), &old, self.ttl_duration())
                .await
                .map_err(storage_err)?;
        }
        let new_token = Self::create_token_value();
        self.storage
            .set(&self.current_key(), &new_token, self.ttl_duration())
            .await
            .map_err(storage_err)?;
        Ok(new_token)
    }
}

// ── Temp-Token ──

/// Default namespace for temp tokens
pub const DEFAULT_TEMP_NAMESPACE: &str = "temp-token";

/// Temp-Token template for short-lived one-time tokens.
#[derive(Clone)]
pub struct TempTokenTemplate {
    storage: Arc<dyn SaStorage>,
    key_prefix: String,
    namespace: String,
}

impl TempTokenTemplate {
    /// Create with default namespace.
    pub fn new(storage: Arc<dyn SaStorage>, key_prefix: impl Into<String>) -> Self {
        Self::with_namespace(storage, key_prefix, DEFAULT_TEMP_NAMESPACE)
    }

    /// Create with explicit namespace.
    pub fn with_namespace(
        storage: Arc<dyn SaStorage>,
        key_prefix: impl Into<String>,
        namespace: impl Into<String>,
    ) -> Self {
        Self {
            storage,
            key_prefix: key_prefix.into(),
            namespace: namespace.into(),
        }
    }

    fn splicing_key(&self, token: &str) -> String {
        format!("{}{}:{}", self.key_prefix, self.namespace, token)
    }

    fn random_token() -> String {
        Uuid::new_v4().simple().to_string()
    }

    /// Create a temp token for the given value.
    pub async fn create_token(
        &self,
        value: impl Into<String>,
        timeout_seconds: i64,
    ) -> Result<String, SaTokenError> {
        let token = Self::random_token();
        self.save_token(&token, value, timeout_seconds).await?;
        Ok(token)
    }

    /// Save a token→value mapping.
    pub async fn save_token(
        &self,
        token: &str,
        value: impl Into<String>,
        timeout_seconds: i64,
    ) -> Result<(), SaTokenError> {
        let ttl = if timeout_seconds > 0 {
            Some(Duration::from_secs(timeout_seconds as u64))
        } else {
            None
        };
        self.storage
            .set(&self.splicing_key(token), &value.into(), ttl)
            .await
            .map_err(storage_err)?;
        Ok(())
    }

    /// Parse a temp token to get its stored value.
    pub async fn parse_token(&self, token: &str) -> Result<Option<String>, SaTokenError> {
        if token.is_empty() {
            return Ok(None);
        }
        self.storage
            .get(&self.splicing_key(token))
            .await
            .map_err(storage_err)
    }

    /// Delete a temp token.
    pub async fn delete_token(&self, token: &str) -> Result<(), SaTokenError> {
        self.storage
            .delete(&self.splicing_key(token))
            .await
            .map_err(storage_err)?;
        Ok(())
    }
}

fn storage_err(e: sa_token_adapter::storage::StorageError) -> SaTokenError {
    SaTokenError::StorageError(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sa_token_storage_memory::MemoryStorage;

    fn make_storage() -> Arc<dyn SaStorage> {
        Arc::new(MemoryStorage::new())
    }

    #[tokio::test]
    async fn test_same_token_create_and_validate() {
        let tpl = SameTokenTemplate::new(make_storage(), "sa:", 3600);
        let token = tpl.get_token().await.unwrap();
        assert!(!token.is_empty());
        assert_eq!(token.len(), SAME_TOKEN_LENGTH);
        assert!(tpl.is_valid(&token).await.unwrap());
    }

    #[tokio::test]
    async fn test_same_token_refresh_rotates() {
        let tpl = SameTokenTemplate::new(make_storage(), "sa:", 3600);
        let first = tpl.get_token().await.unwrap();
        let second = tpl.refresh_token().await.unwrap();
        assert_ne!(first, second);
        assert!(tpl.is_valid(&first).await.unwrap());
        assert!(tpl.is_valid(&second).await.unwrap());
    }

    #[tokio::test]
    async fn test_same_token_invalid_rejected() {
        let tpl = SameTokenTemplate::new(make_storage(), "sa:", 3600);
        tpl.get_token().await.unwrap();
        assert!(!tpl.is_valid("wrong-token").await.unwrap());
        assert!(tpl.check_token("wrong-token").await.is_err());
    }

    #[tokio::test]
    async fn test_same_token_empty_rejected() {
        let tpl = SameTokenTemplate::new(make_storage(), "sa:", 3600);
        assert!(!tpl.is_valid("").await.unwrap());
    }

    #[tokio::test]
    async fn test_temp_token_create_and_parse() {
        let tpl = TempTokenTemplate::new(make_storage(), "sa:");
        let token = tpl.create_token("my-value", 3600).await.unwrap();
        assert!(!token.is_empty());
        let value = tpl.parse_token(&token).await.unwrap();
        assert_eq!(value.as_deref(), Some("my-value"));
    }

    #[tokio::test]
    async fn test_temp_token_delete() {
        let tpl = TempTokenTemplate::new(make_storage(), "sa:");
        let token = tpl.create_token("v1", 3600).await.unwrap();
        tpl.delete_token(&token).await.unwrap();
        assert!(tpl.parse_token(&token).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_temp_token_parse_empty_returns_none() {
        let tpl = TempTokenTemplate::new(make_storage(), "sa:");
        assert!(tpl.parse_token("").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_temp_token_parse_unknown_returns_none() {
        let tpl = TempTokenTemplate::new(make_storage(), "sa:");
        assert!(tpl.parse_token("nonexistent").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_temp_token_namespace_isolation() {
        let storage = make_storage();
        let tpl_a = TempTokenTemplate::with_namespace(storage.clone(), "sa:", "ns-a");
        let tpl_b = TempTokenTemplate::with_namespace(storage, "sa:", "ns-b");
        let token = tpl_a.create_token("value-a", 3600).await.unwrap();
        assert!(tpl_b.parse_token(&token).await.unwrap().is_none());
        assert_eq!(tpl_a.parse_token(&token).await.unwrap().as_deref(), Some("value-a"));
    }
}
