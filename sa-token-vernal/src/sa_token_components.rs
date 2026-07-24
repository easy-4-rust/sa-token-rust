//! Sa-Token 原生对象的 Vernal 组件包。

use std::{error::Error, sync::Arc};

use sa_token_core::{PathAuthConfig, SaTokenManager};
use vernal_context::VernalApplicationBuilder;
use vernal_ioc::{ComponentDefinition, DefinitionError};

use crate::VernalSaTokenBridge;

type BridgeFactoryError = Box<dyn Error + Send + Sync + 'static>;

/// 将 `SaTokenManager` 与 Vernal 安全桥原子装入应用上下文。
///
/// Sa-Token-Rust 继续拥有 Token、Session、角色、权限、存储与路由鉴权语义；
/// Vernal 只管理显式组件身份和依赖顺序。管理器作为应用已经构造好的 Rust 原生
/// 对象注册，不要求实现 Vernal 专用 trait，也不写入进程级全局变量。
#[derive(Clone)]
pub struct SaTokenComponents {
    manager: Arc<SaTokenManager>,
    path_auth: Option<PathAuthConfig>,
}

impl SaTokenComponents {
    /// 使用调用方已经构造完成的 Sa-Token Manager 创建组件包。
    #[must_use]
    pub fn new(manager: Arc<SaTokenManager>) -> Self {
        Self {
            manager,
            path_auth: None,
        }
    }

    /// 为生成的 `VernalSaTokenBridge` 配置路径鉴权规则。
    #[must_use]
    pub fn with_path_auth(mut self, path_auth: PathAuthConfig) -> Self {
        self.path_auth = Some(path_auth);
        self
    }

    /// 返回组件包持有的原始 Manager 共享句柄。
    #[must_use]
    pub const fn manager(&self) -> &Arc<SaTokenManager> {
        &self.manager
    }

    /// 生成 `SaTokenManager -> VernalSaTokenBridge` 显式组件定义。
    ///
    /// Manager 保留调用方传入的 `Arc` 身份，适合让 Web Plugin、后台任务和
    /// Vernal Context 观察同一份登录状态。安全桥是 Container Singleton，其工厂
    /// 只能读取已经声明的 Manager 依赖，解析失败会保留为结构化构造错误。
    #[must_use]
    pub fn definitions(&self) -> [ComponentDefinition; 2] {
        let manager_definition = ComponentDefinition::shared_arc(Arc::clone(&self.manager));
        let path_auth = self.path_auth.clone();

        // 桥接器不从全局变量取 Manager；受限 Resolver 会校验该读取与下方
        // depends_on 声明一致，使依赖图在应用启动前即可完成验证。
        let bridge_definition =
            ComponentDefinition::try_singleton::<VernalSaTokenBridge, _>(move |resolver| {
                let manager = resolver
                    .resolve::<SaTokenManager>()
                    .map_err(|source| Box::new(source) as BridgeFactoryError)?;
                Ok(match path_auth.clone() {
                    Some(path_auth) => VernalSaTokenBridge::with_path_auth(manager, path_auth),
                    None => VernalSaTokenBridge::new(manager),
                })
            })
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
        application.register_all(self.definitions())
    }
}
