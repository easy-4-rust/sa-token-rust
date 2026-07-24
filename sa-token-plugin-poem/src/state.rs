// Author: 金书记
//
//! Sa-Token state management for Poem
//! Poem 的 Sa-Token 状态管理

use sa_token_adapter::storage::SaStorage;
use sa_token_core::SaTokenManager;
use std::sync::Arc;

/// Sa-Token state for Poem framework
/// Poem 框架的 Sa-Token 状态
#[derive(Clone)]
pub struct SaTokenState {
    pub manager: Arc<SaTokenManager>,
}

impl SaTokenState {
    /// Create new Sa-Token state | 创建新的 Sa-Token 状态
    pub fn new(manager: Arc<SaTokenManager>) -> Self {
        Self { manager }
    }

    /// Create builder for Sa-Token state | 创建 Sa-Token 状态构建器
    pub fn builder() -> SaTokenStateBuilder {
        SaTokenStateBuilder::new()
    }
}

/// Builder for Sa-Token state | Sa-Token 状态构建器
pub struct SaTokenStateBuilder {
    storage: Option<Arc<dyn SaStorage>>,
    timeout: Option<i64>,
    token_name: Option<String>,
}

impl SaTokenStateBuilder {
    /// Create new builder | 创建新的构建器
    pub fn new() -> Self {
        Self {
            storage: None,
            timeout: None,
            token_name: None,
        }
    }

    /// Set storage | 设置存储
    pub fn storage(mut self, storage: Arc<dyn SaStorage>) -> Self {
        self.storage = Some(storage);
        self
    }

    /// Set timeout in seconds | 设置超时时间（秒）
    pub fn timeout(mut self, timeout: i64) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Set token name | 设置 token 名称
    pub fn token_name(mut self, name: impl Into<String>) -> Self {
        self.token_name = Some(name.into());
        self
    }

    /// Build Sa-Token state | 构建 Sa-Token 状态
    pub fn build(self) -> sa_token_core::SaTokenResult<SaTokenState> {
        let mut config = sa_token_core::SaTokenConfig::default();

        if let Some(timeout) = self.timeout {
            config.timeout = timeout;
        }

        if let Some(token_name) = self.token_name {
            config.token_name = token_name;
        }

        let storage = match self.storage {
            Some(storage) => storage,
            None => {
                #[cfg(feature = "memory")]
                {
                    Arc::new(sa_token_storage_memory::MemoryStorage::new())
                }
                #[cfg(not(feature = "memory"))]
                {
                    return Err(sa_token_core::SaTokenError::ConfigError(
                        "Poem state requires explicit storage when `memory` is disabled"
                            .to_string(),
                    ));
                }
            }
        };

        let manager = SaTokenManager::new(storage, config);

        Ok(SaTokenState::new(Arc::new(manager)))
    }
}

impl Default for SaTokenStateBuilder {
    fn default() -> Self {
        Self::new()
    }
}
