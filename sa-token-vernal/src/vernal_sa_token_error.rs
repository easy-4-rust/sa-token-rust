//! Sa-Token 与 Vernal 桥接错误对象。

use sa_token_core::SaTokenError;
use vernal_web::{ProblemDetails, ProblemKind, WebFailure};

/// 桥接层可稳定映射到 Web/RPC 协议的失败。
#[derive(Debug, thiserror::Error)]
pub enum VernalSaTokenError {
    /// Sa-Token 路由规则要求身份，但 Token 缺失或无效。
    #[error("authentication is required")]
    Unauthorized,
    /// AOP 调用没有携带 Vernal 请求上下文。
    #[error("Vernal request context is unavailable")]
    MissingRequestContext,
    /// 请求上下文没有携带 owned HTTP 快照。
    #[error("Vernal HTTP request snapshot is unavailable")]
    MissingRequestSnapshot,
    /// Sa-Token 存储或身份数据读取失败。
    #[error("Sa-Token authentication infrastructure failed")]
    Infrastructure(#[source] SaTokenError),
}

impl VernalSaTokenError {
    /// 返回不会泄漏 Token、存储键或内部错误的稳定消息。
    #[must_use]
    pub const fn safe_message(&self) -> &'static str {
        match self {
            Self::Unauthorized => "Authentication is required",
            Self::MissingRequestContext => "Vernal request context is unavailable",
            Self::MissingRequestSnapshot => "Vernal HTTP request snapshot is unavailable",
            Self::Infrastructure(_) => "Authentication service is unavailable",
        }
    }

    /// 返回建议的 HTTP 状态码。
    #[must_use]
    pub const fn status(&self) -> u16 {
        match self {
            Self::Unauthorized => 401,
            Self::MissingRequestContext | Self::MissingRequestSnapshot => 500,
            Self::Infrastructure(_) => 500,
        }
    }

    /// 转换成 Vernal Adapter 可识别的脱敏协议失败。
    ///
    /// Sa-Token 基础设施错误仅保留在服务端 `source` 链；客户端只能看到稳定
    /// `ProblemDetails`，不会得到 Token、存储键或底层错误文本。
    #[must_use]
    pub fn into_web_failure(self) -> WebFailure {
        match self {
            Self::Unauthorized => WebFailure::new(ProblemDetails::new(
                ProblemKind::Unauthenticated,
                401,
                "Authentication is required",
            )),
            Self::MissingRequestContext => WebFailure::new(ProblemDetails::new(
                ProblemKind::Infrastructure,
                500,
                "Vernal request context is unavailable",
            )),
            Self::MissingRequestSnapshot => WebFailure::new(ProblemDetails::new(
                ProblemKind::Infrastructure,
                500,
                "Vernal HTTP request snapshot is unavailable",
            )),
            Self::Infrastructure(source) => WebFailure::with_source(
                ProblemDetails::new(
                    ProblemKind::Infrastructure,
                    500,
                    "Authentication service is unavailable",
                ),
                source,
            ),
        }
    }
}

impl From<SaTokenError> for VernalSaTokenError {
    fn from(error: SaTokenError) -> Self {
        Self::Infrastructure(error)
    }
}
