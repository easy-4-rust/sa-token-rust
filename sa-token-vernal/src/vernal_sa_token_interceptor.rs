//! Sa-Token Vernal 认证拦截器对象。

use std::sync::Arc;

use vernal_aop::{Interceptor, Invocation, InvocationError, InvocationFuture, Next};
use vernal_http::HttpRequestSnapshot;
use vernal_web::RequestContext;

use crate::{VernalSaTokenBridge, VernalSaTokenError};

/// 在 Vernal 环绕调用链中执行 Sa-Token-Rust 认证。
///
/// 拦截器只读取 AOP 调用携带的 owned `RequestContext` 与
/// `HttpRequestSnapshot`，不借用具体 Web 框架 Request。认证成功后，完整下游
/// Future 都运行在 Sa-Token 的 Tokio task-local 上下文中；认证拒绝则不推进
/// `Next`，由 Axum/Tonic Adapter 映射为原生 401 或 gRPC Status。
#[derive(Clone)]
pub struct VernalSaTokenInterceptor {
    bridge: Arc<VernalSaTokenBridge>,
}

impl VernalSaTokenInterceptor {
    /// 创建复用指定安全桥的认证拦截器。
    #[must_use]
    pub fn new(bridge: Arc<VernalSaTokenBridge>) -> Self {
        Self { bridge }
    }

    /// 返回拦截器持有的桥接器。
    #[must_use]
    pub const fn bridge(&self) -> &Arc<VernalSaTokenBridge> {
        &self.bridge
    }
}

impl Interceptor for VernalSaTokenInterceptor {
    fn intercept<'a>(
        &'a self,
        invocation: Arc<Invocation>,
        next: Next<'a>,
    ) -> InvocationFuture<'a> {
        Box::pin(async move {
            let request_context = invocation
                .context()
                .get::<Arc<RequestContext>>()
                .await
                .ok_or_else(|| {
                    InvocationError::target(
                        VernalSaTokenError::MissingRequestContext.into_web_failure(),
                    )
                })?;
            let snapshot = request_context
                .extensions()
                .get::<HttpRequestSnapshot>()
                .await
                .ok_or_else(|| {
                    InvocationError::target(
                        VernalSaTokenError::MissingRequestSnapshot.into_web_failure(),
                    )
                })?;

            let authentication = self
                .bridge
                .authenticate(&snapshot, &request_context)
                .await
                .map_err(|error| InvocationError::target(error.into_web_failure()))?;

            authentication.run(next.run(invocation)).await
        })
    }
}
