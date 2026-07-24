//! Sa-Token Vernal 认证拦截器对象。

use std::sync::Arc;

use vernal_aop::{
    Interceptor, Invocation, InvocationError, InvocationFuture, LocalInterceptor,
    LocalInvocationError, LocalInvocationFuture, LocalNext, Next,
};
use vernal_http::HttpRequestSnapshot;
use vernal_web::RequestContext;

use crate::{VernalAuthentication, VernalSaTokenBridge, VernalSaTokenError, VernalSaTokenPolicy};

/// 在 Vernal 环绕调用链中执行 Sa-Token-Rust 认证。
///
/// 拦截器只读取 AOP 调用携带的 owned `RequestContext` 与
/// `HttpRequestSnapshot`，不借用具体 Web 框架 Request。认证成功后，完整下游
/// Future 都运行在 Sa-Token 的 Tokio task-local 上下文中；认证拒绝则不推进
/// `Next`，由 Axum/Tonic Adapter 映射为原生 401 或 gRPC Status。
#[derive(Clone)]
pub struct VernalSaTokenInterceptor {
    bridge: Arc<VernalSaTokenBridge>,
    policy: Arc<VernalSaTokenPolicy>,
}

impl VernalSaTokenInterceptor {
    /// 创建复用指定安全桥的认证拦截器。
    #[must_use]
    pub fn new(bridge: Arc<VernalSaTokenBridge>) -> Self {
        Self {
            bridge,
            policy: VernalSaTokenPolicy::empty_shared(),
        }
    }

    /// 创建同时执行认证与操作级授权的拦截器。
    #[must_use]
    pub fn with_policy(bridge: Arc<VernalSaTokenBridge>, policy: Arc<VernalSaTokenPolicy>) -> Self {
        Self { bridge, policy }
    }

    /// 返回拦截器持有的桥接器。
    #[must_use]
    pub const fn bridge(&self) -> &Arc<VernalSaTokenBridge> {
        &self.bridge
    }

    /// 返回拦截器持有的不可变操作授权策略。
    #[must_use]
    pub const fn policy(&self) -> &Arc<VernalSaTokenPolicy> {
        &self.policy
    }

    /// 从框架中立调用上下文完成一次认证与操作授权。
    ///
    /// Send-AOP 与 Local-AOP 只在最终目标 Future 的线程移动能力上不同；安全
    /// 决策必须完全一致。该方法集中读取 owned 请求快照、投影 Principal 并执行
    /// 不可变 Operation 策略，防止两条执行平面产生鉴权漂移。
    async fn authenticate_and_authorize(
        &self,
        invocation: &Arc<Invocation>,
    ) -> Result<VernalAuthentication, VernalSaTokenError> {
        let request_context = invocation
            .context()
            .get::<Arc<RequestContext>>()
            .await
            .ok_or(VernalSaTokenError::MissingRequestContext)?;
        let snapshot = request_context
            .extensions()
            .get::<HttpRequestSnapshot>()
            .await
            .ok_or(VernalSaTokenError::MissingRequestSnapshot)?;

        let authentication = self
            .bridge
            .authenticate(&snapshot, &request_context)
            .await?;

        // 认证成功后再按不可变 Operation 策略授权。匿名操作没有规则时继续执行；
        // 一旦声明角色或权限，缺少身份得到 401，身份不足得到 403。
        let principal = request_context.principal().await;
        self.policy
            .authorize(
                invocation.operation(),
                self.bridge.manager(),
                principal.as_deref(),
            )
            .await?;

        Ok(authentication)
    }
}

impl Interceptor for VernalSaTokenInterceptor {
    fn intercept<'a>(
        &'a self,
        invocation: Arc<Invocation>,
        next: Next<'a>,
    ) -> InvocationFuture<'a> {
        Box::pin(async move {
            let authentication = self
                .authenticate_and_authorize(&invocation)
                .await
                .map_err(|error| InvocationError::target(error.into_web_failure()))?;

            authentication.run(next.run(invocation)).await
        })
    }
}

impl LocalInterceptor for VernalSaTokenInterceptor {
    fn intercept_local<'a>(
        &'a self,
        invocation: Arc<Invocation>,
        next: LocalNext<'a>,
    ) -> LocalInvocationFuture<'a> {
        Box::pin(async move {
            let authentication = self
                .authenticate_and_authorize(&invocation)
                .await
                .map_err(|error| LocalInvocationError::target(error.into_web_failure()))?;

            // SaTokenContext 基于 Tokio task-local，能够覆盖非 Send 的 Actix/Ntex
            // 下游 Future；Local-AOP 只改变目标所有权，不改变安全上下文语义。
            authentication.run(next.run(invocation)).await
        })
    }
}
