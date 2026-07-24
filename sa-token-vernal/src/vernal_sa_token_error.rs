//! Sa-Token 与 Vernal 桥接错误对象。

use sa_token_core::SaTokenError;

/// 桥接层可稳定映射到 Web/RPC 协议的失败。
#[derive(Debug, thiserror::Error)]
pub enum VernalSaTokenError {
    /// Sa-Token 路由规则要求身份，但 Token 缺失或无效。
    #[error("authentication is required")]
    Unauthorized,
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
            Self::Infrastructure(_) => "Authentication service is unavailable",
        }
    }

    /// 返回建议的 HTTP 状态码。
    #[must_use]
    pub const fn status(&self) -> u16 {
        match self {
            Self::Unauthorized => 401,
            Self::Infrastructure(_) => 500,
        }
    }
}

impl From<SaTokenError> for VernalSaTokenError {
    fn from(error: SaTokenError) -> Self {
        Self::Infrastructure(error)
    }
}
