// Author: 金书记
//
//! # sa-token-storage-memory
//!
//! 内存存储实现
//!
//! 适用于：
//! - 开发测试环境
//! - 单机部署
//! - 不需要持久化的场景

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sa_token_adapter::storage::{
    Expiration, SaStorage, ScanPage, StorageError, StorageResult, TtlState,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// 内存存储项
#[derive(Debug, Clone)]
struct StorageItem {
    value: String,
    expire_at: Option<DateTime<Utc>>,
}

impl StorageItem {
    fn new(value: String, ttl: Option<Duration>) -> Self {
        let expire_at = ttl.map(|d| Utc::now() + chrono::Duration::from_std(d).unwrap());
        Self { value, expire_at }
    }

    fn is_expired(&self) -> bool {
        if let Some(expire_at) = self.expire_at {
            Utc::now() > expire_at
        } else {
            false
        }
    }
}

/// 内存存储实现
#[derive(Debug, Clone)]
pub struct MemoryStorage {
    data: Arc<RwLock<HashMap<String, StorageItem>>>,
}

impl MemoryStorage {
    /// 创建新的内存存储
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 清理过期的数据
    pub async fn cleanup_expired(&self) {
        let mut data = self.data.write().await;
        data.retain(|_, item| !item.is_expired());
    }
}

impl Default for MemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SaStorage for MemoryStorage {
    async fn get(&self, key: &str) -> StorageResult<Option<String>> {
        let data = self.data.read().await;

        if let Some(item) = data.get(key) {
            if item.is_expired() {
                // 数据已过期
                drop(data);
                self.delete(key).await?;
                Ok(None)
            } else {
                Ok(Some(item.value.clone()))
            }
        } else {
            Ok(None)
        }
    }

    async fn set(&self, key: &str, value: &str, ttl: Option<Duration>) -> StorageResult<()> {
        let mut data = self.data.write().await;
        let item = StorageItem::new(value.to_string(), ttl);
        data.insert(key.to_string(), item);
        Ok(())
    }

    async fn set_with_expiration(
        &self,
        key: &str,
        value: &str,
        expiration: Expiration,
    ) -> StorageResult<()> {
        let mut data = self.data.write().await;
        let expire_at = match expiration {
            Expiration::Persistent => None,
            Expiration::After(ttl) => Some(
                Utc::now()
                    + chrono::Duration::from_std(ttl)
                        .map_err(|error| StorageError::OperationFailed(error.to_string()))?,
            ),
            Expiration::Keep => data.get(key).and_then(|item| item.expire_at),
        };
        data.insert(
            key.to_string(),
            StorageItem {
                value: value.to_string(),
                expire_at,
            },
        );
        Ok(())
    }

    async fn set_if_absent(
        &self,
        key: &str,
        value: &str,
        expiration: Expiration,
    ) -> StorageResult<bool> {
        let mut data = self.data.write().await;
        if data.get(key).is_some_and(|item| item.is_expired()) {
            data.remove(key);
        }
        if data.contains_key(key) {
            return Ok(false);
        }

        let ttl = match expiration {
            Expiration::After(ttl) => Some(ttl),
            Expiration::Persistent | Expiration::Keep => None,
        };
        data.insert(key.to_string(), StorageItem::new(value.to_string(), ttl));
        Ok(true)
    }

    async fn delete(&self, key: &str) -> StorageResult<()> {
        let mut data = self.data.write().await;
        data.remove(key);
        Ok(())
    }

    async fn exists(&self, key: &str) -> StorageResult<bool> {
        let data = self.data.read().await;
        if let Some(item) = data.get(key) {
            Ok(!item.is_expired())
        } else {
            Ok(false)
        }
    }

    async fn expire(&self, key: &str, ttl: Duration) -> StorageResult<()> {
        let mut data = self.data.write().await;
        if let Some(item) = data.get_mut(key) {
            item.expire_at = Some(Utc::now() + chrono::Duration::from_std(ttl).unwrap());
        }
        Ok(())
    }

    async fn ttl(&self, key: &str) -> StorageResult<Option<Duration>> {
        let data = self.data.read().await;
        if let Some(item) = data.get(key) {
            if let Some(expire_at) = item.expire_at {
                let now = Utc::now();
                if expire_at > now {
                    let duration = (expire_at - now)
                        .to_std()
                        .map_err(|e| StorageError::InternalError(e.to_string()))?;
                    Ok(Some(duration))
                } else {
                    Ok(Some(Duration::from_secs(0)))
                }
            } else {
                Ok(None) // 永不过期
            }
        } else {
            Ok(None) // 键不存在
        }
    }

    async fn ttl_state(&self, key: &str) -> StorageResult<TtlState> {
        let data = self.data.read().await;
        let Some(item) = data.get(key) else {
            return Ok(TtlState::Missing);
        };
        if item.is_expired() {
            return Ok(TtlState::Missing);
        }
        match item.expire_at {
            Some(expire_at) => {
                let remaining = (expire_at - Utc::now()).to_std().unwrap_or(Duration::ZERO);
                Ok(TtlState::ExpiresIn(remaining))
            }
            None => Ok(TtlState::Persistent),
        }
    }

    async fn increment_by(&self, key: &str, delta: i64) -> StorageResult<i64> {
        let mut data = self.data.write().await;
        if data.get(key).is_some_and(|item| item.is_expired()) {
            data.remove(key);
        }

        let (current, expire_at) = match data.get(key) {
            Some(item) => (
                item.value.parse::<i64>().map_err(|error| {
                    StorageError::OperationFailed(format!(
                        "value for key '{key}' is not an integer: {error}"
                    ))
                })?,
                item.expire_at,
            ),
            None => (0, None),
        };
        let next = current.checked_add(delta).ok_or_else(|| {
            StorageError::OperationFailed(format!("integer overflow for key '{key}'"))
        })?;
        data.insert(
            key.to_string(),
            StorageItem {
                value: next.to_string(),
                expire_at,
            },
        );
        Ok(next)
    }

    async fn clear(&self) -> StorageResult<()> {
        let mut data = self.data.write().await;
        data.clear();
        Ok(())
    }

    async fn keys(&self, pattern: &str) -> StorageResult<Vec<String>> {
        let data = self.data.read().await;
        let mut result = Vec::new();

        // 将模式转换为正则表达式
        let pattern = pattern.replace("*", ".*");
        let regex = match regex::Regex::new(&pattern) {
            Ok(r) => r,
            Err(e) => {
                return Err(StorageError::OperationFailed(format!(
                    "Invalid pattern: {}",
                    e
                )));
            }
        };

        // 筛选匹配的键
        for (key, item) in data.iter() {
            if !item.is_expired() && regex.is_match(key) {
                result.push(key.clone());
            }
        }

        Ok(result)
    }

    async fn scan(
        &self,
        pattern: &str,
        cursor: Option<&str>,
        limit: usize,
    ) -> StorageResult<ScanPage> {
        if limit == 0 {
            return Err(StorageError::OperationFailed(
                "scan limit must be greater than zero".to_string(),
            ));
        }
        let offset = cursor.unwrap_or("0").parse::<usize>().map_err(|error| {
            StorageError::OperationFailed(format!("invalid scan cursor: {error}"))
        })?;
        let regex = regex::Regex::new(&pattern.replace("*", ".*"))
            .map_err(|error| StorageError::OperationFailed(format!("Invalid pattern: {error}")))?;
        let data = self.data.read().await;
        let mut keys: Vec<String> = data
            .iter()
            .filter(|(key, item)| !item.is_expired() && regex.is_match(key))
            .map(|(key, _)| key.clone())
            .collect();
        keys.sort();

        let page_keys: Vec<String> = keys.iter().skip(offset).take(limit).cloned().collect();
        let next_offset = offset.saturating_add(page_keys.len());
        let next_cursor = (next_offset < keys.len()).then(|| next_offset.to_string());
        Ok(ScanPage {
            keys: page_keys,
            next_cursor,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_storage() {
        let storage = MemoryStorage::new();

        // 测试设置和获取
        storage.set("key1", "value1", None).await.unwrap();
        let value = storage.get("key1").await.unwrap();
        assert_eq!(value, Some("value1".to_string()));

        // 测试删除
        storage.delete("key1").await.unwrap();
        let value = storage.get("key1").await.unwrap();
        assert_eq!(value, None);

        // 测试存在性
        storage.set("key2", "value2", None).await.unwrap();
        assert!(storage.exists("key2").await.unwrap());
        assert!(!storage.exists("key3").await.unwrap());
    }

    #[tokio::test]
    async fn test_ttl() {
        let storage = MemoryStorage::new();

        // 设置带过期时间的键
        storage
            .set("key1", "value1", Some(Duration::from_secs(1)))
            .await
            .unwrap();

        // 立即获取应该成功
        let value = storage.get("key1").await.unwrap();
        assert_eq!(value, Some("value1".to_string()));

        // 等待过期
        tokio::time::sleep(Duration::from_secs(2)).await;

        // 过期后应该返回 None
        let value = storage.get("key1").await.unwrap();
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn set_if_absent_is_atomic_under_concurrency() {
        let storage = Arc::new(MemoryStorage::new());
        let mut tasks = Vec::new();
        for index in 0..32 {
            let storage = storage.clone();
            tasks.push(tokio::spawn(async move {
                storage
                    .set_if_absent(
                        "nonce",
                        &index.to_string(),
                        Expiration::After(Duration::from_secs(30)),
                    )
                    .await
                    .unwrap()
            }));
        }

        let mut inserted = 0;
        for task in tasks {
            inserted += usize::from(task.await.unwrap());
        }
        assert_eq!(inserted, 1);
    }

    #[tokio::test]
    async fn increment_by_is_atomic_and_preserves_ttl() {
        let storage = Arc::new(MemoryStorage::new());
        storage
            .set("counter", "0", Some(Duration::from_secs(30)))
            .await
            .unwrap();
        let mut tasks = Vec::new();
        for _ in 0..50 {
            let storage = storage.clone();
            tasks.push(tokio::spawn(async move {
                storage.increment_by("counter", 1).await.unwrap()
            }));
        }
        for task in tasks {
            task.await.unwrap();
        }

        assert_eq!(storage.get("counter").await.unwrap().as_deref(), Some("50"));
        assert!(matches!(
            storage.ttl_state("counter").await.unwrap(),
            TtlState::ExpiresIn(_)
        ));
    }

    #[tokio::test]
    async fn scan_returns_stable_pages() {
        let storage = MemoryStorage::new();
        for key in ["auth:3", "auth:1", "other:1", "auth:2"] {
            storage.set(key, "value", None).await.unwrap();
        }

        let first = storage.scan("auth:*", None, 2).await.unwrap();
        assert_eq!(first.keys, vec!["auth:1", "auth:2"]);
        let second = storage
            .scan("auth:*", first.next_cursor.as_deref(), 2)
            .await
            .unwrap();
        assert_eq!(second.keys, vec!["auth:3"]);
        assert!(second.next_cursor.is_none());
    }
}
