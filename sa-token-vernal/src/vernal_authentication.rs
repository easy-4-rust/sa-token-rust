//! Vernal 请求对应的 Sa-Token 认证结果对象。

use std::future::Future;

use sa_token_core::{AuthFlowResult, TokenValue};

/// 已完成认证、可把业务 Future 放入 Sa-Token 请求级上下文的结果。
///
/// 该对象拥有 `AuthFlowResult`，调用 `run` 后会在完整 Future 生命周期内安装
/// `SaTokenContext`。这保证 `StpUtil` 等 API 在跨 `.await` 和 Tokio Worker
/// 切换时仍读取到当前请求身份，而不是依赖全局 Service Locator。
pub struct VernalAuthentication {
    flow: AuthFlowResult,
}

impl VernalAuthentication {
    pub(crate) const fn new(flow: AuthFlowResult) -> Self {
        Self { flow }
    }

    /// 返回已经校验的登录标识。
    #[must_use]
    pub fn login_id(&self) -> Option<&str> {
        self.flow.login_id.as_deref()
    }

    /// 返回请求携带且已经过 Sa-Token 流程处理的 Token。
    #[must_use]
    pub const fn token(&self) -> Option<&TokenValue> {
        self.flow.token.as_ref()
    }

    /// 当前请求是否具有已认证登录身份。
    #[must_use]
    pub const fn is_authenticated(&self) -> bool {
        self.flow.login_id.is_some()
    }

    /// 在当前认证请求的 `SaTokenContext` 中运行下游 Future。
    pub async fn run<F, Output>(self, future: F) -> Output
    where
        F: Future<Output = Output>,
    {
        self.flow.run(future).await
    }
}
