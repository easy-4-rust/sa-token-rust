// Author: 金书记
//
//! 存储适配器trait定义

use async_trait::async_trait;
use std::time::Duration;
use thiserror::Error;

pub type StorageResult<T> = Result<T, StorageError>;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Storage operation failed: {0}")]
    OperationFailed(String),

    #[error("Key not found: {0}")]
    KeyNotFound(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("Internal error: {0}")]
    InternalError(String),

    #[error("Storage operation is not supported: {0}")]
    UnsupportedOperation(&'static str),
}

/// Expiration semantics for writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Expiration {
    /// Store without expiration.
    Persistent,
    /// Expire after the supplied duration.
    After(Duration),
    /// Preserve the existing key's expiration when replacing its value.
    Keep,
}

impl From<Option<Duration>> for Expiration {
    fn from(ttl: Option<Duration>) -> Self {
        match ttl {
            Some(ttl) => Self::After(ttl),
            None => Self::Persistent,
        }
    }
}

/// Unambiguous key TTL state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TtlState {
    Missing,
    Persistent,
    ExpiresIn(Duration),
}

/// One page returned by a non-blocking key scan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanPage {
    pub keys: Vec<String>,
    /// Opaque backend cursor. `None` means the scan is complete.
    pub next_cursor: Option<String>,
}

/// 存储适配器trait
///
/// 所有存储实现（内存、Redis、数据库等）都需要实现这个trait
#[async_trait]
pub trait SaStorage: Send + Sync {
    /// 获取值
    async fn get(&self, key: &str) -> StorageResult<Option<String>>;

    /// 设置值
    ///
    /// # 参数
    /// * `key` - 键
    /// * `value` - 值
    /// * `ttl` - 过期时间（None表示永不过期）
    async fn set(&self, key: &str, value: &str, ttl: Option<Duration>) -> StorageResult<()>;

    /// Set with explicit expiration semantics.
    async fn set_with_expiration(
        &self,
        key: &str,
        value: &str,
        expiration: Expiration,
    ) -> StorageResult<()> {
        match expiration {
            Expiration::Persistent => self.set(key, value, None).await,
            Expiration::After(ttl) => self.set(key, value, Some(ttl)).await,
            Expiration::Keep => Err(StorageError::UnsupportedOperation(
                "set_with_expiration(Keep)",
            )),
        }
    }

    /// Atomically insert a key only when it does not already exist.
    ///
    /// Returns `true` when the value was inserted and `false` when the key
    /// already existed.
    async fn set_if_absent(
        &self,
        _key: &str,
        _value: &str,
        _expiration: Expiration,
    ) -> StorageResult<bool> {
        Err(StorageError::UnsupportedOperation("set_if_absent"))
    }

    /// 删除值
    async fn delete(&self, key: &str) -> StorageResult<()>;

    /// 检查键是否存在
    async fn exists(&self, key: &str) -> StorageResult<bool>;

    /// 设置过期时间
    async fn expire(&self, key: &str, ttl: Duration) -> StorageResult<()>;

    /// 获取剩余过期时间
    async fn ttl(&self, key: &str) -> StorageResult<Option<Duration>>;

    /// Get TTL without conflating missing and persistent keys.
    async fn ttl_state(&self, key: &str) -> StorageResult<TtlState> {
        if !self.exists(key).await? {
            return Ok(TtlState::Missing);
        }
        match self.ttl(key).await? {
            Some(ttl) => Ok(TtlState::ExpiresIn(ttl)),
            None => Ok(TtlState::Persistent),
        }
    }

    /// 批量获取
    async fn mget(&self, keys: &[&str]) -> StorageResult<Vec<Option<String>>> {
        let mut results = Vec::with_capacity(keys.len());
        for key in keys {
            results.push(self.get(key).await?);
        }
        Ok(results)
    }

    /// 批量设置
    async fn mset(&self, items: &[(&str, &str)], ttl: Option<Duration>) -> StorageResult<()> {
        for (key, value) in items {
            self.set(key, value, ttl).await?;
        }
        Ok(())
    }

    /// 批量删除
    async fn mdel(&self, keys: &[&str]) -> StorageResult<()> {
        for key in keys {
            self.delete(key).await?;
        }
        Ok(())
    }

    /// 原子递增
    async fn incr(&self, key: &str) -> StorageResult<i64> {
        self.increment_by(key, 1).await
    }

    /// 原子递减
    async fn decr(&self, key: &str) -> StorageResult<i64> {
        self.increment_by(key, -1).await
    }

    /// Atomically add `delta`, preserving an existing key's TTL.
    async fn increment_by(&self, _key: &str, _delta: i64) -> StorageResult<i64> {
        Err(StorageError::UnsupportedOperation("increment_by"))
    }

    /// 清空所有数据（谨慎使用）
    async fn clear(&self) -> StorageResult<()>;

    /// 获取匹配模式的所有键
    ///
    /// # 参数
    /// * `pattern` - 匹配模式，支持 * 通配符
    async fn keys(&self, _pattern: &str) -> StorageResult<Vec<String>> {
        Err(StorageError::UnsupportedOperation("keys"))
    }

    /// Incrementally scan matching keys without requiring a blocking KEYS call.
    async fn scan(
        &self,
        _pattern: &str,
        _cursor: Option<&str>,
        _limit: usize,
    ) -> StorageResult<ScanPage> {
        Err(StorageError::UnsupportedOperation("scan"))
    }
}
