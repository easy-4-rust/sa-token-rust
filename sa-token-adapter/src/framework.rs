// Author: 金书记
//
//! 框架集成适配器trait定义

use async_trait::async_trait;

/// 框架适配器trait
///
/// 用于定义框架特定的集成逻辑
#[async_trait]
pub trait FrameworkAdapter: Send + Sync {
    /// 框架名称
    fn name(&self) -> &str;

    /// 初始化框架集成
    async fn initialize(&self) -> Result<(), String>;

    /// 清理资源
    async fn shutdown(&self) -> Result<(), String> {
        Ok(())
    }
}
