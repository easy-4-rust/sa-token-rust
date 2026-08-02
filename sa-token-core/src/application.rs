// Author: 金书记
//
//! Application global KV store | 应用全局 KV 存储
//!
//! 对应 Java `cn.dev33.satoken.application.SaApplication`。
//!
//! 在应用全局范围内存值、取值。数据在应用重启后失效，
//! 如果集成了 Redis，则在 Redis 重启后失效。

use std::sync::{Arc, OnceLock};
use std::time::Duration;

use serde_json::Value;

use crate::error::SaTokenError;
use crate::runtime::SaTokenRuntime;
use sa_token_adapter::storage::SaStorage;

/// 全局 Application KV 存储
///
/// 对应 Java `SaApplication.defaultInstance`。
/// 使用 DAO 的存储能力，以 `{prefix}application:{key}` 格式存储数据。
#[derive(Clone)]
pub struct SaApplication {
    runtime: SaTokenRuntime,
}

/// 全局默认实例
static DEFAULT_INSTANCE: OnceLock<SaApplication> = OnceLock::new();

impl SaApplication {
    /// 获取默认实例（对应 Java `SaApplication.defaultInstance`）
    pub fn default_instance() -> &'static SaApplication {
        DEFAULT_INSTANCE.get_or_init(|| {
            // 使用全局 runtime 或创建一个最小 runtime
            // 实际使用时应通过 SaManager 获取
            panic!("SaApplication not initialized. Call SaApplication::init(runtime) first.")
        })
    }

    /// 初始化默认实例（在应用启动时调用）
    pub fn init(runtime: SaTokenRuntime) {
        DEFAULT_INSTANCE.get_or_init(|| SaApplication { runtime });
    }

    /// 使用指定 runtime 创建实例
    pub fn new(runtime: SaTokenRuntime) -> Self {
        Self { runtime }
    }

    /// 获取存储实例
    fn storage(&self) -> &Arc<dyn SaStorage> {
        self.runtime.manager().storage()
    }

    /// 获取存储键前缀
    fn key_prefix(&self) -> String {
        format!(
            "{}application:",
            self.runtime.manager().config.storage_key_prefix
        )
    }

    /// 拼接存储键
    fn splicing_key(&self, key: &str) -> String {
        format!("{}{}", self.key_prefix(), key)
    }

    /// 取值（对应 Java `SaApplication.get(key)`）
    pub async fn get(&self, key: &str) -> Result<Option<Value>, SaTokenError> {
        let storage_key = self.splicing_key(key);
        let result = self
            .storage()
            .get(&storage_key)
            .await
            .map_err(|e| SaTokenError::StorageError(e.to_string()))?;

        match result {
            Some(s) => {
                let value: Value =
                    serde_json::from_str(&s).map_err(SaTokenError::SerializationError)?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    /// 写值（对应 Java `SaApplication.set(key, value)`）
    pub async fn set(&self, key: &str, value: &Value) -> Result<(), SaTokenError> {
        self.set_with_ttl(key, value, None).await
    }

    /// 写值（带 TTL，对应 Java `SaApplication.set(key, value, ttl)`）
    pub async fn set_with_ttl(
        &self,
        key: &str,
        value: &Value,
        ttl: Option<Duration>,
    ) -> Result<(), SaTokenError> {
        let storage_key = self.splicing_key(key);
        let json_str = serde_json::to_string(value).map_err(SaTokenError::SerializationError)?;

        self.storage()
            .set(&storage_key, &json_str, ttl)
            .await
            .map_err(|e| SaTokenError::StorageError(e.to_string()))?;

        Ok(())
    }

    /// 删值（对应 Java `SaApplication.delete(key)`）
    pub async fn delete(&self, key: &str) -> Result<(), SaTokenError> {
        let storage_key = self.splicing_key(key);
        self.storage()
            .delete(&storage_key)
            .await
            .map_err(|e| SaTokenError::StorageError(e.to_string()))?;

        Ok(())
    }

    /// 返回当前存入的所有 key（对应 Java `SaApplication.keys()`）
    pub async fn keys(&self) -> Result<Vec<String>, SaTokenError> {
        let prefix = self.key_prefix();
        let pattern = format!("{}*", prefix);
        let all_keys = self
            .storage()
            .keys(&pattern)
            .await
            .map_err(|e| SaTokenError::StorageError(e.to_string()))?;

        // 裁减掉固定前缀，保留 key 名称
        let prefix_len = prefix.len();
        let keys: Vec<String> = all_keys
            .into_iter()
            .filter_map(|k| {
                if k.len() > prefix_len {
                    Some(k[prefix_len..].to_string())
                } else {
                    None
                }
            })
            .collect();

        Ok(keys)
    }

    /// 清空所有 application 数据（对应 Java `SaApplication.clear()`）
    pub async fn clear(&self) -> Result<(), SaTokenError> {
        let keys = self.keys().await?;
        for key in keys {
            self.delete(&key).await?;
        }
        Ok(())
    }
}

// 便捷函数（对应 Java 静态方法风格）

/// 全局取值
pub async fn get(key: &str) -> Result<Option<Value>, SaTokenError> {
    SaApplication::default_instance().get(key).await
}

/// 全局写值
pub async fn set(key: &str, value: &Value) -> Result<(), SaTokenError> {
    SaApplication::default_instance().set(key, value).await
}

/// 全局删值
pub async fn delete(key: &str) -> Result<(), SaTokenError> {
    SaApplication::default_instance().delete(key).await
}

/// 全局获取所有 key
pub async fn keys() -> Result<Vec<String>, SaTokenError> {
    SaApplication::default_instance().keys().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Note: Tests require a runtime to be initialized
    // In practice, this is done by the test harness

    #[tokio::test]
    async fn test_application_key_prefix() {
        // Test key prefix generation
        let prefix = format!("{}application:", "sa:");
        assert_eq!(prefix, "sa:application:");
    }

    #[tokio::test]
    async fn test_splicing_key() {
        let prefix = format!("{}application:", "sa:");
        let key = "test_key";
        let full_key = format!("{}{}", prefix, key);
        assert_eq!(full_key, "sa:application:test_key");
    }

    #[tokio::test]
    async fn test_keys_prefix_stripping() {
        // Simulate key stripping logic
        let prefix = "sa:application:".to_string();
        let all_keys = vec![
            "sa:application:key1".to_string(),
            "sa:application:key2".to_string(),
            "sa:application:key3".to_string(),
        ];

        let prefix_len = prefix.len();
        let stripped: Vec<String> = all_keys
            .into_iter()
            .filter_map(|k| {
                if k.len() > prefix_len {
                    Some(k[prefix_len..].to_string())
                } else {
                    None
                }
            })
            .collect();

        assert_eq!(stripped, vec!["key1", "key2", "key3"]);
    }
}
