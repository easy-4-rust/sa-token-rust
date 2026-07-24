//! Sa-Token 原生对象的 Vernal 组件包。

use std::sync::Arc;

use sa_token_core::{PathAuthConfig, SaTokenManager};
use vernal_aop::Advisor;
use vernal_context::VernalApplicationBuilder;
use vernal_ioc::{ComponentDefinition, DefinitionError};

use crate::{VernalSaTokenBridge, VernalSaTokenInterceptor, VernalSaTokenPointcut};

/// 将 `SaTokenManager` 与 Vernal 安全桥原子装入应用上下文。
///
/// Sa-Token-Rust 继续拥有 Token、Session、角色、权限、存储与路由鉴权语义；
/// Vernal 只管理显式组件身份和依赖顺序。管理器作为应用已经构造好的 Rust 原生
/// 对象注册，不要求实现 Vernal 专用 trait，也不写入进程级全局变量。
#[derive(Clone)]
pub struct SaTokenComponents {
    bridge: Arc<VernalSaTokenBridge>,
    advisor_order: i32,
}

impl SaTokenComponents {
    /// 使用调用方已经构造完成的 Sa-Token Manager 创建组件包。
    #[must_use]
    pub fn new(manager: Arc<SaTokenManager>) -> Self {
        Self {
            bridge: Arc::new(VernalSaTokenBridge::new(manager)),
            advisor_order: -1000,
        }
    }

    /// 为生成的 `VernalSaTokenBridge` 配置路径鉴权规则。
    #[must_use]
    pub fn with_path_auth(mut self, path_auth: PathAuthConfig) -> Self {
        self.bridge = Arc::new(VernalSaTokenBridge::with_path_auth(
            Arc::clone(self.bridge.manager()),
            path_auth,
        ));
        self
    }

    /// 设置认证 Advisor 顺序；数值越小越早进入调用链。
    #[must_use]
    pub const fn with_advisor_order(mut self, advisor_order: i32) -> Self {
        self.advisor_order = advisor_order;
        self
    }

    /// 返回组件包持有的原始 Manager 共享句柄。
    #[must_use]
    pub fn manager(&self) -> &Arc<SaTokenManager> {
        self.bridge.manager()
    }

    /// 返回组件包与认证 Advisor 共用的桥接器实例。
    #[must_use]
    pub const fn bridge(&self) -> &Arc<VernalSaTokenBridge> {
        &self.bridge
    }

    /// 生成 `SaTokenManager -> VernalSaTokenBridge` 显式组件定义。
    ///
    /// Manager 与 Bridge 都保留组件包持有的精确 `Arc` 身份，适合让 Web Plugin、
    /// 后台任务、Vernal Context 和认证 Advisor 观察同一份登录状态。Bridge 定义
    /// 仍显式声明对 Manager 的依赖，使启动校验与诊断图保持完整。
    #[must_use]
    pub fn definitions(&self) -> [ComponentDefinition; 2] {
        let manager_definition = ComponentDefinition::shared_arc(Arc::clone(self.bridge.manager()));

        // Bridge 与 Advisor 共用同一个 Arc；显式 depends_on 仍把 Manager -> Bridge
        // 关系写入 Vernal 图，使启动前校验和诊断保持完整。
        let bridge_definition = ComponentDefinition::shared_arc(Arc::clone(&self.bridge))
            .depends_on::<SaTokenManager>();

        [manager_definition, bridge_definition]
    }

    /// 将 Sa-Token 组件包原子安装到高层 Vernal 应用建造器。
    ///
    /// # Errors
    ///
    /// 应用已注册 `SaTokenManager` 或 `VernalSaTokenBridge` 时返回
    /// [`DefinitionError`]；失败不会留下部分组件定义。
    pub fn install<'a>(
        &self,
        application: &'a mut VernalApplicationBuilder,
    ) -> Result<&'a mut VernalApplicationBuilder, DefinitionError> {
        application.register_all(self.definitions())?;
        application.advisor(Advisor::new(
            VernalSaTokenPointcut,
            VernalSaTokenInterceptor::new(Arc::clone(&self.bridge)),
            self.advisor_order,
        ));
        Ok(application)
    }
}
