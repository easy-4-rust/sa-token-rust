//! Vernal ApplicationEnvironment 到 Sa-Token 原生配置建造器的绑定器。

use sa_token_core::{
    LogoutMode, LogoutRange, ReplacedLoginExitMode, ReplacedRange, SaTokenConfig,
    config::{SaTokenConfigBuilder, TokenStyle},
};
use vernal_context::ApplicationEnvironment;

use crate::VernalSaTokenConfigError;

/// 从 Vernal 的不可变应用环境向 Sa-Token Builder 应用显式配置。
///
/// 默认前缀是 `sa-token`，因此 `sa-token.timeout=7200` 会调用 Sa-Token 自己的
/// `SaTokenConfigBuilder::timeout(7200)`。Vernal 只负责属性来源优先级、占位符展开
/// 和类型转换；默认值、配置对象、Storage、Listener、Manager 与 Runtime 的所有权
/// 仍属于 Sa-Token-Rust。
///
/// 绑定器不会枚举或记录 Environment 中的任意键，也不会把配置值写进 `Debug` 或
/// 错误文本。未声明的属性保持 Sa-Token 原生默认值；调用方仍需显式向返回的 Builder
/// 安装 `SaStorage` 与 Listener 后再构建 Runtime。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VernalSaTokenConfigBinder {
    prefix: String,
}

impl VernalSaTokenConfigBinder {
    /// 创建使用 `sa-token` 前缀的绑定器。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 创建自定义前缀的绑定器。
    ///
    /// # Errors
    ///
    /// 前缀为空、包含空白/控制字符，或以点号开头/结尾时返回
    /// [`VernalSaTokenConfigError::InvalidPrefix`]。
    pub fn with_prefix(prefix: impl Into<String>) -> Result<Self, VernalSaTokenConfigError> {
        let prefix = prefix.into();
        if prefix.is_empty()
            || prefix.starts_with('.')
            || prefix.ends_with('.')
            || prefix.chars().any(char::is_whitespace)
            || prefix.chars().any(char::is_control)
        {
            return Err(VernalSaTokenConfigError::InvalidPrefix);
        }
        Ok(Self { prefix })
    }

    /// 返回绑定器使用的属性前缀。
    #[must_use]
    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    /// 把 Environment 中存在的属性应用到调用方提供的 Sa-Token Builder。
    ///
    /// Storage 和 Listener 不属于字符串配置，本方法绝不隐式创建它们。调用方可以先
    /// 绑定环境配置，再通过 Sa-Token 原生 Builder 安装这些运行时对象。
    ///
    /// # Errors
    ///
    /// 属性读取、占位符展开、类型转换或枚举值解析失败时返回
    /// [`VernalSaTokenConfigError`]；错误文本不会包含实际属性值。
    pub fn bind(
        &self,
        environment: &ApplicationEnvironment,
        mut builder: SaTokenConfigBuilder,
    ) -> Result<SaTokenConfigBuilder, VernalSaTokenConfigError> {
        if let Some(value) = self.string(environment, "token-name")? {
            builder = builder.token_name(value);
        }
        if let Some(value) = self.value::<i64>(environment, "timeout")? {
            builder = builder.timeout(value);
        }
        if let Some(value) = self.value::<i64>(environment, "active-timeout")? {
            builder = builder.active_timeout(value);
        }
        if let Some(value) = self.value::<bool>(environment, "dynamic-active-timeout")? {
            builder = builder.dynamic_active_timeout(value);
        }
        if let Some(value) = self.value::<bool>(environment, "auto-renew")? {
            builder = builder.auto_renew(value);
        }
        if let Some(value) = self.value::<bool>(environment, "is-concurrent")? {
            builder = builder.is_concurrent(value);
        }
        if let Some(value) = self.value::<bool>(environment, "is-share")? {
            builder = builder.is_share(value);
        }
        if let Some(value) = self.string(environment, "token-style")? {
            builder = builder.token_style(self.token_style("token-style", &value)?);
        }
        if let Some(value) = self.string(environment, "token-prefix")? {
            builder = builder.token_prefix(value);
        }
        if let Some(value) = self.string(environment, "storage-key-prefix")? {
            builder = builder.storage_key_prefix(value);
        }
        if let Some(value) = self.string(environment, "jwt-secret-key")? {
            builder = builder.jwt_secret_key(value);
        }
        if let Some(value) = self.string(environment, "jwt-algorithm")? {
            builder = builder.jwt_algorithm(value);
        }
        if let Some(value) = self.string(environment, "jwt-issuer")? {
            builder = builder.jwt_issuer(value);
        }
        if let Some(value) = self.string(environment, "jwt-audience")? {
            builder = builder.jwt_audience(value);
        }
        if let Some(value) = self.value::<bool>(environment, "jwt-fallback-on-error")? {
            builder = builder.jwt_fallback_on_error(value);
        }
        if let Some(value) = self.value::<bool>(environment, "enable-nonce")? {
            builder = builder.enable_nonce(value);
        }
        if let Some(value) = self.value::<i64>(environment, "nonce-timeout")? {
            builder = builder.nonce_timeout(value);
        }
        if let Some(value) = self.value::<bool>(environment, "enable-refresh-token")? {
            builder = builder.enable_refresh_token(value);
        }
        if let Some(value) = self.value::<i64>(environment, "refresh-token-timeout")? {
            builder = builder.refresh_token_timeout(value);
        }
        if let Some(value) = self.value::<i64>(environment, "max-login-count")? {
            builder = builder.max_login_count(value);
        }
        if let Some(value) = self.string(environment, "overflow-logout-mode")? {
            builder =
                builder.overflow_logout_mode(self.logout_mode("overflow-logout-mode", &value)?);
        }
        if let Some(value) = self.string(environment, "replaced-login-exit-mode")? {
            builder = builder.replaced_login_exit_mode(
                self.replaced_login_exit_mode("replaced-login-exit-mode", &value)?,
            );
        }
        if let Some(value) = self.string(environment, "replaced-range")? {
            builder = builder.replaced_range(self.replaced_range("replaced-range", &value)?);
        }
        if let Some(value) = self.value::<bool>(environment, "right-now-create-token-session")? {
            builder = builder.right_now_create_token_session(value);
        }
        if let Some(value) = self.value::<bool>(environment, "token-session-check-login")? {
            builder = builder.token_session_check_login(value);
        }
        if let Some(value) = self.string(environment, "logout-range")? {
            builder = builder.logout_range(self.logout_range("logout-range", &value)?);
        }
        if let Some(value) = self.value::<bool>(environment, "is-logout-keep-token-session")? {
            builder = builder.is_logout_keep_token_session(value);
        }
        if let Some(value) = self.value::<bool>(environment, "is-lasting-cookie")? {
            builder = builder.is_lasting_cookie(value);
        }
        if let Some(value) = self.value::<bool>(environment, "is-write-header")? {
            builder = builder.is_write_header(value);
        }
        if let Some(value) = self.value::<bool>(environment, "is-logout-keep-freeze-ops")? {
            builder = builder.is_logout_keep_freeze_ops(value);
        }
        if let Some(value) = self.value::<bool>(environment, "cookie-auto-fill-prefix")? {
            builder = builder.cookie_auto_fill_prefix(value);
        }
        if let Some(value) = self.value::<bool>(environment, "is-print")? {
            builder = builder.is_print(value);
        }
        if let Some(value) = self.string(environment, "log-level")? {
            builder = builder.log_level(value);
        }
        if let Some(value) = self.value::<i32>(environment, "log-level-int")? {
            builder = builder.log_level_int(value);
        }
        if let Some(value) = self.value::<bool>(environment, "is-color-log")? {
            builder = builder.is_color_log(Some(value));
        }
        if let Some(value) = self.string(environment, "http-basic")? {
            builder = builder.http_basic(value);
        }
        if let Some(value) = self.string(environment, "http-digest")? {
            builder = builder.http_digest(value);
        }
        if let Some(value) = self.string(environment, "curr-domain")? {
            builder = builder.curr_domain(value);
        }
        if let Some(value) = self.value::<i64>(environment, "same-token-timeout")? {
            builder = builder.same_token_timeout(value);
        }
        if let Some(value) = self.value::<bool>(environment, "check-same-token")? {
            builder = builder.check_same_token(value);
        }
        if let Some(value) = self.value::<i32>(environment, "data-refresh-period")? {
            builder = builder.data_refresh_period(value);
        }
        if let Some(value) = self.value::<i32>(environment, "max-try-times")? {
            builder = builder.max_try_times(value);
        }
        Ok(builder)
    }

    /// 直接生成不含 Storage 和 Listener 的 Sa-Token 原生配置快照。
    ///
    /// 该快捷方法适合在装配阶段验证配置或把结果交给
    /// `SaTokenManager::new(storage, config)`；它不会安装全局 Runtime。
    ///
    /// # Errors
    ///
    /// 与 [`Self::bind`] 相同。
    pub fn build_config(
        &self,
        environment: &ApplicationEnvironment,
    ) -> Result<SaTokenConfig, VernalSaTokenConfigError> {
        Ok(self
            .bind(environment, SaTokenConfig::builder())?
            .build_config())
    }

    /// 拼接一个完整属性键；前缀已在构造阶段校验。
    fn key(&self, suffix: &str) -> String {
        format!("{}.{suffix}", self.prefix)
    }

    /// 读取字符串属性并保留 Vernal 的优先级与占位符语义。
    fn string(
        &self,
        environment: &ApplicationEnvironment,
        suffix: &str,
    ) -> Result<Option<String>, VernalSaTokenConfigError> {
        environment.property(&self.key(suffix)).map_err(Into::into)
    }

    /// 读取任意 `FromStr` 属性；错误由 Environment 记录键、来源和目标类型。
    fn value<T>(
        &self,
        environment: &ApplicationEnvironment,
        suffix: &str,
    ) -> Result<Option<T>, VernalSaTokenConfigError>
    where
        T: std::str::FromStr,
    {
        environment.get(&self.key(suffix)).map_err(Into::into)
    }

    /// 把枚举文本统一为小写短横线形式，兼容常见的 snake_case 配置写法。
    fn normalized_choice(value: &str) -> String {
        value.trim().to_ascii_lowercase().replace('_', "-")
    }

    fn invalid_choice(&self, suffix: &str, expected: &'static str) -> VernalSaTokenConfigError {
        VernalSaTokenConfigError::InvalidChoice {
            key: self.key(suffix),
            expected,
        }
    }

    fn token_style(
        &self,
        suffix: &str,
        value: &str,
    ) -> Result<TokenStyle, VernalSaTokenConfigError> {
        match Self::normalized_choice(value).as_str() {
            "uuid" => Ok(TokenStyle::Uuid),
            "simple-uuid" => Ok(TokenStyle::SimpleUuid),
            "random-32" => Ok(TokenStyle::Random32),
            "random-64" => Ok(TokenStyle::Random64),
            "random-128" => Ok(TokenStyle::Random128),
            "jwt" => Ok(TokenStyle::Jwt),
            "hash" => Ok(TokenStyle::Hash),
            "timestamp" => Ok(TokenStyle::Timestamp),
            "tik" => Ok(TokenStyle::Tik),
            _ => Err(self.invalid_choice(
                suffix,
                "uuid, simple-uuid, random-32, random-64, random-128, jwt, hash, timestamp, tik",
            )),
        }
    }

    fn logout_mode(
        &self,
        suffix: &str,
        value: &str,
    ) -> Result<LogoutMode, VernalSaTokenConfigError> {
        match Self::normalized_choice(value).as_str() {
            "logout" => Ok(LogoutMode::Logout),
            "kick-out" => Ok(LogoutMode::KickOut),
            "replaced" => Ok(LogoutMode::Replaced),
            _ => Err(self.invalid_choice(suffix, "logout, kick-out, replaced")),
        }
    }

    fn replaced_login_exit_mode(
        &self,
        suffix: &str,
        value: &str,
    ) -> Result<ReplacedLoginExitMode, VernalSaTokenConfigError> {
        match Self::normalized_choice(value).as_str() {
            "old-device" => Ok(ReplacedLoginExitMode::OldDevice),
            "new-device" => Ok(ReplacedLoginExitMode::NewDevice),
            _ => Err(self.invalid_choice(suffix, "old-device, new-device")),
        }
    }

    fn replaced_range(
        &self,
        suffix: &str,
        value: &str,
    ) -> Result<ReplacedRange, VernalSaTokenConfigError> {
        match Self::normalized_choice(value).as_str() {
            "curr-device-type" | "current-device-type" => Ok(ReplacedRange::CurrDeviceType),
            "all-device-type" => Ok(ReplacedRange::AllDeviceType),
            _ => Err(self.invalid_choice(
                suffix,
                "curr-device-type, current-device-type, all-device-type",
            )),
        }
    }

    fn logout_range(
        &self,
        suffix: &str,
        value: &str,
    ) -> Result<LogoutRange, VernalSaTokenConfigError> {
        match Self::normalized_choice(value).as_str() {
            "token" => Ok(LogoutRange::Token),
            "account" => Ok(LogoutRange::Account),
            _ => Err(self.invalid_choice(suffix, "token, account")),
        }
    }
}

impl Default for VernalSaTokenConfigBinder {
    fn default() -> Self {
        Self {
            prefix: "sa-token".to_owned(),
        }
    }
}
