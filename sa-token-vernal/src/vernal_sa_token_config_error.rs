//! Vernal Environment 绑定 Sa-Token 配置时的结构化错误。

use vernal_context::EnvironmentError;

/// Vernal 属性无法安全映射为 Sa-Token 原生配置。
///
/// 错误文本只包含属性键和允许的枚举集合，从不包含实际属性值。这样即使失败的键是
/// `jwt-secret-key`、`http-basic` 或 `http-digest`，日志与启动诊断也不会泄露密钥。
/// 原始 [`EnvironmentError`] 仅通过标准错误链供服务端受控诊断使用。
#[derive(Debug, thiserror::Error)]
pub enum VernalSaTokenConfigError {
    /// 配置前缀无法组成合法的 Vernal 属性键。
    #[error("invalid Sa-Token property prefix")]
    InvalidPrefix,

    /// Vernal Environment 读取或类型转换失败。
    #[error("failed to read Sa-Token property from Vernal Environment")]
    Environment(#[source] EnvironmentError),

    /// 字符串属性不是目标 Sa-Token 枚举支持的值。
    #[error("invalid choice for property `{key}`; expected one of: {expected}")]
    InvalidChoice {
        /// 发生错误的完整属性键。
        key: String,
        /// 静态、无敏感信息的允许值列表。
        expected: &'static str,
    },
}

impl From<EnvironmentError> for VernalSaTokenConfigError {
    fn from(error: EnvironmentError) -> Self {
        Self::Environment(error)
    }
}
