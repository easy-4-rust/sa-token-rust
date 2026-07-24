// Author: 金书记
//
//! 配置模块

use crate::event::SaTokenListener;
use sa_token_adapter::storage::SaStorage;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

/// sa-token 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaTokenConfig {
    /// Token 名称（例如在 header 或 cookie 中的键名）
    pub token_name: String,

    /// Token 有效期（秒），-1 表示永久有效
    pub timeout: i64,

    /// Token 最低活跃频率（秒），-1 表示不限制
    ///
    /// 超过该间隔未活跃则 token 冻结（`TokenInactive`）；配合 `auto_renew` 时亦用于续签时长。
    pub active_timeout: i64,

    /// 是否启用 per-token 动态 active_timeout（对齐 Java dynamicActiveTimeout，Phase2 完善）
    pub dynamic_active_timeout: bool,

    /// 是否开启自动续签（默认 true，对齐 Java SaTokenConfig）
    ///
    /// 如果设置为 true，在以下场景会自动续签 token：
    /// - 调用 get_token_info() 时
    /// - 中间件验证 token 时
    /// - 调用无参数的 StpUtil 方法时
    ///
    /// 续签时长由 active_timeout 决定：
    /// - 如果 active_timeout > 0，则续签 active_timeout 秒
    /// - 如果 active_timeout <= 0，则续签 timeout 秒
    pub auto_renew: bool,

    /// 是否允许同一账号并发登录
    pub is_concurrent: bool,

    /// 在多人登录同一账号时，是否共享一个 token（默认 false，对齐 Java）
    pub is_share: bool,

    /// Token 风格（uuid、simple-uuid、random-32、random-64、random-128）
    pub token_style: TokenStyle,

    /// Token 前缀（HTTP header 中 token 值的固定前缀，例如 "Bearer "）。
    /// 对应 Java `SaTokenConfig.tokenPrefix`。
    /// 默认 None 表示无前缀，前端提交时直接传 token 值。
    pub token_prefix: Option<String>,

    /// 是否输出操作日志
    pub is_log: bool,

    /// 是否从 cookie 中读取 token
    pub is_read_cookie: bool,

    /// 是否从 header 中读取 token
    pub is_read_header: bool,

    /// 是否从请求体中读取 token
    pub is_read_body: bool,

    /// JWT 密钥（如果使用 JWT）
    pub jwt_secret_key: Option<String>,

    /// JWT 算法（默认 HS256）
    pub jwt_algorithm: Option<String>,

    /// JWT 签发者
    pub jwt_issuer: Option<String>,

    /// JWT 受众
    pub jwt_audience: Option<String>,

    /// JWT 生成失败时是否回退为 UUID（默认 true）；失败时始终 `tracing::warn`
    pub jwt_fallback_on_error: bool,

    /// 是否启用防重放攻击（nonce 机制）
    pub enable_nonce: bool,

    /// Nonce 有效期（秒），-1 表示使用 token timeout
    pub nonce_timeout: i64,

    /// 是否启用 Refresh Token
    pub enable_refresh_token: bool,

    /// Refresh Token 有效期（秒），默认 7 天
    pub refresh_token_timeout: i64,

    /// 存储键前缀（用于 Redis/数据库等存储后端的键命名）
    /// 默认 "sa:"，所有存储键将以此为前缀，如 "sa:token:"、"sa:session:" 等
    /// 注意：此字段与 token_prefix（HTTP header 中的 Bearer 前缀）不同
    pub storage_key_prefix: String,

    /// 同一账号最大登录数量，-1 表示不限制
    pub max_login_count: i64,

    /// 超出 max_login_count 时的下线模式
    pub overflow_logout_mode: LogoutMode,

    /// 非并发顶号时：踢旧设备还是拒绝新登录
    pub replaced_login_exit_mode: ReplacedLoginExitMode,

    /// 顶号范围：当前设备类型或全部设备
    pub replaced_range: ReplacedRange,

    /// 登录时是否立即创建 Token-Session
    pub right_now_create_token_session: bool,

    /// 获取 Token-Session 时是否校验 token 登录态
    pub token_session_check_login: bool,

    /// 默认 logout 范围（预留）
    pub logout_range: LogoutRange,

    /// logout 时是否保留 Token-Session
    pub is_logout_keep_token_session: bool,

    // ── 以下字段对齐 Java SaTokenConfig，补充 Wave-1 配置补齐 ──
    /// 是否为持久 Cookie（临时 Cookie 在浏览器关闭时自动删除）
    /// 对应 Java `isLastingCookie`，默认 true
    pub is_lasting_cookie: bool,

    /// 是否在登录后将 token 写入响应头
    /// 对应 Java `isWriteHeader`，默认 false
    pub is_write_header: bool,

    /// 如果 token 已被冻结，是否保留其操作权（是否允许此 token 调用注销 API）
    /// 对应 Java `isLogoutKeepFreezeOps`，默认 false
    pub is_logout_keep_freeze_ops: bool,

    /// Cookie 模式是否自动填充 token 前缀
    /// 对应 Java `cookieAutoFillPrefix`，默认 false
    pub cookie_auto_fill_prefix: bool,

    /// 是否在初始化配置时在控制台打印版本字符画
    /// 对应 Java `isPrint`，默认 true
    pub is_print: bool,

    /// 日志等级（trace/debug/info/warn/error/fatal）
    /// 对应 Java `logLevel`，默认 "trace"
    pub log_level: String,

    /// 日志等级 int 值（1=trace, 2=debug, 3=info, 4=warn, 5=error, 6=fatal）
    /// 对应 Java `logLevelInt`，默认 1
    pub log_level_int: i32,

    /// 是否打印彩色日志（None = 自动检测终端）
    /// 对应 Java `isColorLog`，默认 None
    pub is_color_log: Option<bool>,

    /// Http Basic 认证的默认账号和密码，冒号隔开，例如 "sa:123456"
    /// 对应 Java `httpBasic`，默认空字符串
    pub http_basic: String,

    /// Http Digest 认证的默认账号和密码
    /// 对应 Java `httpDigest`，默认空字符串
    pub http_digest: String,

    /// 配置当前项目的网络访问地址
    /// 对应 Java `currDomain`
    pub curr_domain: Option<String>,

    /// Same-Token 的有效期（秒），用于微服务 RPC 鉴权
    /// 对应 Java `sameTokenTimeout`，默认 86400（1 天）
    pub same_token_timeout: i64,

    /// 是否校验 Same-Token（部分 rpc 插件有效）
    /// 对应 Java `checkSameToken`，默认 false
    pub check_same_token: bool,

    /// 默认 DAO 实现中每次清理过期数据间隔的时间（秒）
    /// 对应 Java `dataRefreshPeriod`，默认 30，-1 不启动定时清理
    pub data_refresh_period: i32,

    /// 在每次创建 token 时的最高循环次数，用于保证 token 唯一性
    /// 对应 Java `maxTryTimes`，默认 12，-1 不循环尝试
    pub max_try_times: i32,
}

impl Default for SaTokenConfig {
    fn default() -> Self {
        Self {
            token_name: "satoken".to_string(),
            timeout: 2592000, // 30天
            active_timeout: -1,
            dynamic_active_timeout: false,
            auto_renew: true,
            is_concurrent: true,
            is_share: false,
            token_style: TokenStyle::Uuid,
            token_prefix: None,
            is_log: false,
            is_read_cookie: true,
            is_read_header: true,
            is_read_body: true,
            jwt_secret_key: None,
            jwt_algorithm: Some("HS256".to_string()),
            jwt_issuer: None,
            jwt_audience: None,
            jwt_fallback_on_error: true,
            enable_nonce: false,
            nonce_timeout: -1,
            enable_refresh_token: false,
            refresh_token_timeout: 604800, // 7 天
            storage_key_prefix: "sa:".to_string(),
            max_login_count: 12,
            overflow_logout_mode: LogoutMode::Logout,
            replaced_login_exit_mode: ReplacedLoginExitMode::OldDevice,
            replaced_range: ReplacedRange::CurrDeviceType,
            right_now_create_token_session: false,
            token_session_check_login: true,
            logout_range: LogoutRange::Token,
            is_logout_keep_token_session: false,
            // ── Wave-1 配置补齐 ──
            is_lasting_cookie: true,
            is_write_header: false,
            is_logout_keep_freeze_ops: false,
            cookie_auto_fill_prefix: false,
            is_print: true,
            log_level: "trace".to_string(),
            log_level_int: 1,
            is_color_log: None,
            http_basic: String::new(),
            http_digest: String::new(),
            curr_domain: None,
            same_token_timeout: 86400,
            check_same_token: false,
            data_refresh_period: 30,
            max_try_times: 12,
        }
    }
}

impl SaTokenConfig {
    pub fn builder() -> SaTokenConfigBuilder {
        SaTokenConfigBuilder::default()
    }

    pub fn timeout_duration(&self) -> Option<Duration> {
        if self.timeout < 0 {
            None
        } else {
            Some(Duration::from_secs(self.timeout as u64))
        }
    }

    /// 构造存储键：拼接 storage_key_prefix 与后缀
    /// 例如：make_key("token:", "abc123") → "sa:token:abc123"
    pub fn make_key(&self, suffix: &str, id: &str) -> String {
        format!("{}{}{}", self.storage_key_prefix, suffix, id)
    }

    /// 获取存储键前缀
    pub fn key_prefix(&self) -> &str {
        &self.storage_key_prefix
    }
}

/// Token 风格 | Token Style
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TokenStyle {
    /// UUID 风格 | UUID style
    Uuid,
    /// 简化的 UUID（去掉横杠）| Simple UUID (without hyphens)
    SimpleUuid,
    /// 32位随机字符串 | 32-character random string
    Random32,
    /// 64位随机字符串 | 64-character random string
    Random64,
    /// 128位随机字符串 | 128-character random string
    Random128,
    /// JWT 风格（JSON Web Token）| JWT style (JSON Web Token)
    Jwt,
    /// Hash 风格（SHA256哈希）| Hash style (SHA256 hash)
    Hash,
    /// 时间戳风格（毫秒级时间戳+随机数）| Timestamp style (millisecond timestamp + random)
    Timestamp,
    /// Tik 风格（Java 兼容的 `2_14_16__`，共 36 字符）
    /// Tik style (Java-compatible `2_14_16__`, 36 characters)
    Tik,
}

/// 下线模式（对齐 Java SaLogoutMode）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LogoutMode {
    #[default]
    Logout,
    KickOut,
    Replaced,
}

/// 非并发顶号时踢旧或拒新
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ReplacedLoginExitMode {
    #[default]
    OldDevice,
    NewDevice,
}

/// 顶号影响范围
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ReplacedRange {
    #[default]
    CurrDeviceType,
    AllDeviceType,
}

/// logout 范围（预留）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LogoutRange {
    #[default]
    Token,
    Account,
}

/// 配置构建器
#[derive(Default)]
pub struct SaTokenConfigBuilder {
    config: SaTokenConfig,
    storage: Option<Arc<dyn SaStorage>>,
    listeners: Vec<Arc<dyn SaTokenListener>>,
}

impl SaTokenConfigBuilder {
    pub fn token_name(mut self, name: impl Into<String>) -> Self {
        self.config.token_name = name.into();
        self
    }

    pub fn timeout(mut self, timeout: i64) -> Self {
        self.config.timeout = timeout;
        self
    }

    pub fn active_timeout(mut self, timeout: i64) -> Self {
        self.config.active_timeout = timeout;
        self
    }

    /// 设置是否启用 per-token 动态 active_timeout
    pub fn dynamic_active_timeout(mut self, enabled: bool) -> Self {
        self.config.dynamic_active_timeout = enabled;
        self
    }

    /// 设置是否开启自动续签
    pub fn auto_renew(mut self, enabled: bool) -> Self {
        self.config.auto_renew = enabled;
        self
    }

    pub fn is_concurrent(mut self, concurrent: bool) -> Self {
        self.config.is_concurrent = concurrent;
        self
    }

    pub fn is_share(mut self, share: bool) -> Self {
        self.config.is_share = share;
        self
    }

    pub fn token_style(mut self, style: TokenStyle) -> Self {
        self.config.token_style = style;
        self
    }

    /// 设置 Token 前缀（HTTP header 中 token 值的固定前缀，例如 "Bearer "）。
    /// 对应 Java `SaTokenConfig.setTokenPrefix(...)`。
    pub fn token_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.config.token_prefix = Some(prefix.into());
        self
    }

    /// 设置存储键前缀（默认 "sa:"）
    ///
    /// 注意：此字段与 token_prefix（HTTP header 中的 Bearer 前缀）不同
    /// 此前缀用于 Redis/数据库等存储后端的键命名
    pub fn storage_key_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.config.storage_key_prefix = prefix.into();
        self
    }

    pub fn jwt_secret_key(mut self, key: impl Into<String>) -> Self {
        self.config.jwt_secret_key = Some(key.into());
        self
    }

    /// 设置 JWT 算法
    pub fn jwt_algorithm(mut self, algorithm: impl Into<String>) -> Self {
        self.config.jwt_algorithm = Some(algorithm.into());
        self
    }

    /// 设置 JWT 签发者
    pub fn jwt_issuer(mut self, issuer: impl Into<String>) -> Self {
        self.config.jwt_issuer = Some(issuer.into());
        self
    }

    /// 设置 JWT 受众
    pub fn jwt_audience(mut self, audience: impl Into<String>) -> Self {
        self.config.jwt_audience = Some(audience.into());
        self
    }

    pub fn jwt_fallback_on_error(mut self, fallback: bool) -> Self {
        self.config.jwt_fallback_on_error = fallback;
        self
    }

    /// 启用防重放攻击（nonce 机制）
    pub fn enable_nonce(mut self, enable: bool) -> Self {
        self.config.enable_nonce = enable;
        self
    }

    /// 设置 Nonce 有效期（秒）
    pub fn nonce_timeout(mut self, timeout: i64) -> Self {
        self.config.nonce_timeout = timeout;
        self
    }

    /// 启用 Refresh Token
    pub fn enable_refresh_token(mut self, enable: bool) -> Self {
        self.config.enable_refresh_token = enable;
        self
    }

    /// 设置 Refresh Token 有效期（秒）
    pub fn refresh_token_timeout(mut self, timeout: i64) -> Self {
        self.config.refresh_token_timeout = timeout;
        self
    }

    pub fn max_login_count(mut self, count: i64) -> Self {
        self.config.max_login_count = count;
        self
    }

    pub fn overflow_logout_mode(mut self, mode: LogoutMode) -> Self {
        self.config.overflow_logout_mode = mode;
        self
    }

    pub fn replaced_login_exit_mode(mut self, mode: ReplacedLoginExitMode) -> Self {
        self.config.replaced_login_exit_mode = mode;
        self
    }

    pub fn replaced_range(mut self, range: ReplacedRange) -> Self {
        self.config.replaced_range = range;
        self
    }

    pub fn right_now_create_token_session(mut self, enabled: bool) -> Self {
        self.config.right_now_create_token_session = enabled;
        self
    }

    pub fn token_session_check_login(mut self, enabled: bool) -> Self {
        self.config.token_session_check_login = enabled;
        self
    }

    pub fn logout_range(mut self, range: LogoutRange) -> Self {
        self.config.logout_range = range;
        self
    }

    pub fn is_logout_keep_token_session(mut self, keep: bool) -> Self {
        self.config.is_logout_keep_token_session = keep;
        self
    }

    // ── Wave-1 配置补齐 builder ──

    /// 设置是否为持久 Cookie（对应 Java `isLastingCookie`）
    pub fn is_lasting_cookie(mut self, lasting: bool) -> Self {
        self.config.is_lasting_cookie = lasting;
        self
    }

    /// 设置是否在登录后写入响应头（对应 Java `isWriteHeader`）
    pub fn is_write_header(mut self, write: bool) -> Self {
        self.config.is_write_header = write;
        self
    }

    /// 设置冻结 token 是否保留操作权（对应 Java `isLogoutKeepFreezeOps`）
    pub fn is_logout_keep_freeze_ops(mut self, keep: bool) -> Self {
        self.config.is_logout_keep_freeze_ops = keep;
        self
    }

    /// 设置 Cookie 是否自动填充 token 前缀（对应 Java `cookieAutoFillPrefix`）
    pub fn cookie_auto_fill_prefix(mut self, auto: bool) -> Self {
        self.config.cookie_auto_fill_prefix = auto;
        self
    }

    /// 设置是否打印启动字符画（对应 Java `isPrint`）
    pub fn is_print(mut self, print: bool) -> Self {
        self.config.is_print = print;
        self
    }

    /// 设置日志等级（对应 Java `logLevel`）
    pub fn log_level(mut self, level: impl Into<String>) -> Self {
        self.config.log_level = level.into();
        self
    }

    /// 设置日志等级 int 值（对应 Java `logLevelInt`）
    pub fn log_level_int(mut self, level: i32) -> Self {
        self.config.log_level_int = level;
        self
    }

    /// 设置是否打印彩色日志（对应 Java `isColorLog`）
    pub fn is_color_log(mut self, color: Option<bool>) -> Self {
        self.config.is_color_log = color;
        self
    }

    /// 设置 HTTP Basic 默认账密（对应 Java `httpBasic`）
    pub fn http_basic(mut self, basic: impl Into<String>) -> Self {
        self.config.http_basic = basic.into();
        self
    }

    /// 设置 HTTP Digest 默认账密（对应 Java `httpDigest`）
    pub fn http_digest(mut self, digest: impl Into<String>) -> Self {
        self.config.http_digest = digest.into();
        self
    }

    /// 设置当前项目网络地址（对应 Java `currDomain`）
    pub fn curr_domain(mut self, domain: impl Into<String>) -> Self {
        self.config.curr_domain = Some(domain.into());
        self
    }

    /// 设置 Same-Token 超时时间（对应 Java `sameTokenTimeout`）
    pub fn same_token_timeout(mut self, timeout: i64) -> Self {
        self.config.same_token_timeout = timeout;
        self
    }

    /// 设置是否校验 Same-Token（对应 Java `checkSameToken`）
    pub fn check_same_token(mut self, check: bool) -> Self {
        self.config.check_same_token = check;
        self
    }

    /// 设置 DAO 过期数据清理间隔（对应 Java `dataRefreshPeriod`）
    pub fn data_refresh_period(mut self, period: i32) -> Self {
        self.config.data_refresh_period = period;
        self
    }

    /// 设置 token 唯一性最大尝试次数（对应 Java `maxTryTimes`）
    pub fn max_try_times(mut self, times: i32) -> Self {
        self.config.max_try_times = times;
        self
    }

    /// 设置存储方式
    pub fn storage(mut self, storage: Arc<dyn SaStorage>) -> Self {
        self.storage = Some(storage);
        self
    }

    /// 注册事件监听器
    ///
    /// 可以多次调用以注册多个监听器
    ///
    /// # 示例
    /// ```rust,ignore
    /// use std::sync::Arc;
    /// use sa_token_core::{SaTokenConfig, SaTokenListener};
    ///
    /// struct MyListener;
    /// impl SaTokenListener for MyListener { /* ... */ }
    ///
    /// let manager = SaTokenConfig::builder()
    ///     .storage(Arc::new(MemoryStorage::new()))
    ///     .register_listener(Arc::new(MyListener))
    ///     .build();
    /// ```
    pub fn register_listener(mut self, listener: Arc<dyn SaTokenListener>) -> Self {
        self.listeners.push(listener);
        self
    }

    /// Build an isolated runtime (storage must be configured).
    ///
    /// 自动完成以下操作：
    /// 1. 创建 SaTokenManager
    /// 2. 注册所有事件监听器
    /// 3. 初始化 StpUtil
    ///
    /// Auto-complete the following operations:
    /// 1. Create SaTokenManager
    /// 2. Register all event listeners
    /// 3. Initialize StpUtil
    ///
    /// # Panics
    /// 如果未设置 storage，会 panic
    ///
    /// # 示例
    /// ```rust,ignore
    /// use std::sync::Arc;
    /// use sa_token_core::SaTokenConfig;
    /// use sa_token_storage_memory::MemoryStorage;
    ///
    /// // 一行代码完成所有初始化！
    /// // Complete all initialization in one line!
    /// let runtime = SaTokenConfig::builder()
    ///     .storage(Arc::new(MemoryStorage::new()))
    ///     .timeout(7200)
    ///     .register_listener(Arc::new(MyListener))
    ///     .build()?;
    ///
    /// // Optional compatibility facade:
    /// runtime.install_global()?;
    /// ```
    pub fn build(self) -> crate::SaTokenResult<crate::SaTokenRuntime> {
        let storage = self.storage.ok_or_else(|| {
            crate::SaTokenError::ConfigError(
                "storage must be configured before building SaTokenRuntime".to_string(),
            )
        })?;
        let manager = crate::SaTokenManager::new(storage, self.config);

        // 同步注册所有监听器
        // Register all listeners synchronously
        if !self.listeners.is_empty() {
            let event_bus = manager.event_bus();
            for listener in self.listeners {
                event_bus.register(listener);
            }
        }

        Ok(crate::SaTokenRuntime::new(manager))
    }

    /// Build and explicitly install the runtime for the legacy global facade.
    pub fn build_and_install_global(self) -> crate::SaTokenResult<crate::SaTokenRuntime> {
        let runtime = self.build()?;
        runtime.install_global()?;
        Ok(runtime)
    }

    /// 仅构建配置（不创建 Manager）
    pub fn build_config(self) -> SaTokenConfig {
        self.config
    }
}
