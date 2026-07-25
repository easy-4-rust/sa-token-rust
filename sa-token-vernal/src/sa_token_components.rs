//! Sa-Token 原生对象的 Vernal 具名应用模块。

use std::{error::Error, sync::Arc};

use sa_token_core::{PathAuthConfig, SaTokenManager};
use vernal_aop::{Advisor, LocalAdvisor};
use vernal_context::{
    ApplicationModule, ApplicationModuleError, ApplicationModuleRegistrar, VernalApplicationBuilder,
};
use vernal_ioc::ComponentDefinition;

use crate::{
    VernalSaTokenBridge, VernalSaTokenInterceptor, VernalSaTokenPointcut, VernalSaTokenPolicy,
};

/// 将 `SaTokenManager`、安全桥、授权策略与双 AOP Advisor 原子装入应用上下文。
///
/// Sa-Token-Rust 继续拥有 Token、Session、角色、权限、存储与路由鉴权语义；
/// Vernal 只管理显式组件身份和依赖顺序。管理器作为应用已经构造好的 Rust 原生
/// 对象注册，不要求实现 Vernal 专用 trait，也不写入进程级全局变量。
///
/// 本对象同时实现 [`ApplicationModule`]。组件 Definition、Send Advisor 与 Local
/// Advisor 先写入隔离 Registrar，只有模块身份与完整 IoC Bundle 都通过预检后才会
/// 一次提交；任何 Definition 冲突都不会遗留半条安全拦截链。
#[derive(Clone)]
pub struct SaTokenComponents {
    bridge: Arc<VernalSaTokenBridge>,
    policy: Arc<VernalSaTokenPolicy>,
    advisor_order: i32,
}

impl SaTokenComponents {
    /// 使用调用方已经构造完成的 Sa-Token Manager 创建组件包。
    #[must_use]
    pub fn new(manager: Arc<SaTokenManager>) -> Self {
        Self {
            bridge: Arc::new(VernalSaTokenBridge::new(manager)),
            policy: VernalSaTokenPolicy::empty_shared(),
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

    /// 安装调用方已经构造完成的操作级授权策略。
    ///
    /// 传入的精确 `Arc` 会同时注册到 Vernal IoC，并交给认证拦截器复用。
    #[must_use]
    pub fn with_authorization_policy(mut self, policy: Arc<VernalSaTokenPolicy>) -> Self {
        self.policy = policy;
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

    /// 返回组件包与认证 Advisor 共用的操作级授权策略。
    #[must_use]
    pub const fn authorization_policy(&self) -> &Arc<VernalSaTokenPolicy> {
        &self.policy
    }

    /// 生成 Manager、Bridge 与授权策略的显式组件定义。
    ///
    /// Manager、Bridge 与 Policy 都保留组件包持有的精确 `Arc` 身份，适合让
    /// Web Plugin、后台任务、Vernal Context 和认证 Advisor 观察同一份登录状态
    /// 与授权配置。Bridge 定义仍显式声明对 Manager 的依赖，使启动校验与诊断图
    /// 保持完整。
    #[must_use]
    pub fn definitions(&self) -> [ComponentDefinition; 3] {
        let manager_definition = ComponentDefinition::shared_arc(Arc::clone(self.bridge.manager()));

        // Bridge、Policy 与 Advisor 共用精确 Arc；显式 depends_on 仍把
        // Manager -> Bridge 关系写入 Vernal 图，使启动前校验和诊断保持完整。
        let bridge_definition = ComponentDefinition::shared_arc(Arc::clone(&self.bridge))
            .depends_on::<SaTokenManager>();
        let policy_definition = ComponentDefinition::shared_arc(Arc::clone(&self.policy));

        [manager_definition, bridge_definition, policy_definition]
    }

    /// 将 Sa-Token 安全模块原子安装到高层 Vernal 应用建造器。
    ///
    /// # Errors
    ///
    /// 模块身份重复，或应用已注册 `SaTokenManager`、`VernalSaTokenBridge`、
    /// `VernalSaTokenPolicy` 时返回 [`ApplicationModuleError`]；失败不会留下
    /// 部分组件定义、Send Advisor、Local Advisor 或已占用的模块身份。
    pub fn install<'a>(
        &self,
        application: &'a mut VernalApplicationBuilder,
    ) -> Result<&'a mut VernalApplicationBuilder, ApplicationModuleError> {
        application.register_module(self.clone())
    }
}

impl ApplicationModule for SaTokenComponents {
    /// 返回参与应用级去重和诊断的稳定安全模块身份。
    fn name(&self) -> &'static str {
        "sa-token.security"
    }

    /// 将组件图和两类安全 Advisor 暂存到隔离 Registrar。
    ///
    /// 本方法不直接修改真实 `VernalApplicationBuilder`。即使 Definition 校验最终
    /// 失败，已经构造的 Advisor 也只存在于随错误一起丢弃的临时模块中。
    fn configure(
        self,
        registrar: &mut ApplicationModuleRegistrar,
    ) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
        registrar.register_all(self.definitions());

        // Send 与 Local 执行平面必须复用同一 Bridge、Policy 和顺序，避免不同 Web
        // Adapter 产生认证或授权语义漂移。
        registrar.advisor(Advisor::new(
            VernalSaTokenPointcut,
            VernalSaTokenInterceptor::with_policy(
                Arc::clone(&self.bridge),
                Arc::clone(&self.policy),
            ),
            self.advisor_order,
        ));
        registrar.local_advisor(LocalAdvisor::new(
            VernalSaTokenPointcut,
            VernalSaTokenInterceptor::with_policy(
                Arc::clone(&self.bridge),
                Arc::clone(&self.policy),
            ),
            self.advisor_order,
        ));

        Ok(())
    }
}
