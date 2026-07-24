//! Sa-Token 认证流与 Vernal 请求上下文的桥接对象。

use std::sync::Arc;

use sa_token_core::{PathAuthConfig, SaTokenManager, run_auth_flow};
use vernal_http::HttpRequestSnapshot;
use vernal_web::{RequestContext, SecurityPrincipal};

use crate::{VernalAuthentication, VernalSaRequest, VernalSaTokenError};

/// 由 Sa-Token-Rust 持有的 Vernal 安全桥。
///
/// Sa-Token-Rust 继续拥有 Token、Session、路由鉴权和角色语义；该对象只把
/// `HttpRequestSnapshot` 接入统一认证流，并把成功身份投影为 Vernal 的只读
/// `SecurityPrincipal`。依赖方向始终是 Sa-Token-Rust -> Vernal。
#[derive(Clone)]
pub struct VernalSaTokenBridge {
    manager: Arc<SaTokenManager>,
    path_config: Option<PathAuthConfig>,
}

impl VernalSaTokenBridge {
    /// 创建“有 Token 则校验、无 Token 则匿名”的桥接器。
    #[must_use]
    pub fn new(manager: Arc<SaTokenManager>) -> Self {
        Self {
            manager,
            path_config: None,
        }
    }

    /// 创建带 Sa-Token 路由鉴权规则的桥接器。
    #[must_use]
    pub fn with_path_auth(manager: Arc<SaTokenManager>, path_config: PathAuthConfig) -> Self {
        Self {
            manager,
            path_config: Some(path_config),
        }
    }

    /// 返回桥接器持有的显式 Sa-Token Manager。
    #[must_use]
    pub const fn manager(&self) -> &Arc<SaTokenManager> {
        &self.manager
    }

    /// 执行 Sa-Token 认证并更新 Vernal 请求主体。
    ///
    /// 匿名成功会清空主体，避免复用上下文时残留旧身份；已登录请求会从
    /// Sa-Token Manager 读取角色并写入 `SecurityPrincipal`。返回值拥有
    /// `SaTokenContext`，调用方应通过 `VernalAuthentication::run` 执行业务
    /// Future。
    pub async fn authenticate(
        &self,
        snapshot: &HttpRequestSnapshot,
        request_context: &RequestContext,
    ) -> Result<VernalAuthentication, VernalSaTokenError> {
        let request = VernalSaRequest::new(snapshot);
        let flow = run_auth_flow(&request, &self.manager, self.path_config.as_ref()).await;
        if flow.should_reject() {
            request_context.set_principal(None).await;
            return Err(VernalSaTokenError::Unauthorized);
        }

        let principal = match flow.login_id.as_deref() {
            Some(login_id) => {
                let roles = self.manager.get_roles(login_id).await?;
                Some(Arc::new(SecurityPrincipal::new(login_id.to_owned(), roles)))
            }
            None => None,
        };
        request_context.set_principal(principal).await;
        Ok(VernalAuthentication::new(flow))
    }
}
