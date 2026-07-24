// Author: 金书记
//
//! # sa-token-core
//!
//! sa-token-rust 的核心库，提供与框架无关的认证授权功能
//!
//! ## 主要功能
//!
//! - Token 管理：生成、验证、刷新
//! - Session 管理：会话存储与管理
//! - 权限验证：基于角色/权限的访问控制
//! - 账号管理：登录、登出、踢人下线、封禁等
//!
//! ## 使用示例
//!
//! ```rust,ignore
//! use sa_token_core::SaTokenManager;
//!
//! let manager = SaTokenManager::new(storage, config);
//! let token = manager.create_token("user_123").await?;
//! ```

pub mod api_security;
pub mod base32;
pub mod config;
pub mod constant_time;
pub mod context;
pub mod disable;
pub mod distributed;
pub mod event;
pub mod firewall;
pub mod http_auth;
pub mod nonce;
pub mod oauth2;
pub mod online;
pub mod permission;
pub mod plugin;
pub mod prelude;
pub mod refresh;
pub mod router;
pub mod runtime;
pub mod safe;
pub mod same_token;
pub mod session;
pub mod sso;
pub mod stp_interface;
pub mod stp_logic;
pub mod telemetry;
pub mod temp_jwt;
pub mod token;
pub mod token_session;
pub mod totp;
pub mod util;
pub mod ws;

pub mod error;
mod manager;

pub use config::{LogoutMode, LogoutRange, ReplacedLoginExitMode, ReplacedRange, SaTokenConfig};
pub use context::SaTokenContext;
pub use error::{SaTokenError, SaTokenResult};
pub use manager::SaTokenManager;
pub use plugin::SaTokenPlugin;
pub use runtime::SaTokenRuntime;
pub use stp_interface::StpInterface;
pub use temp_jwt::TempJwtTemplate;
pub use totp::{Totp, TotpAlgorithm, TotpError};
pub use util::{LoginId, StpUtil};

// 重新导出核心类型
pub use api_security::{ApiKeyConfig, ApiKeyModel, ApiKeyTemplate, SignConfig, SignTemplate};
pub use disable::{
    DEFAULT_DISABLE_LEVEL, DEFAULT_DISABLE_SERVICE, MIN_DISABLE_LEVEL, NOT_DISABLE_LEVEL,
};
pub use distributed::{
    DistributedSession, DistributedSessionManager, DistributedSessionStorage,
    InMemoryDistributedStorage, SaStorageDistributedStorage, ServiceCredential,
};
pub use event::{
    LoggingListener, SaTokenEvent, SaTokenEventBus, SaTokenEventType, SaTokenListener,
};
pub use firewall::{FirewallStrategy, GlobalFirewall, has_non_printable_ascii, is_path_valid};
pub use http_auth::{HttpBasicAccount, HttpBasicTemplate, HttpDigestModel, HttpDigestTemplate};
pub use nonce::NonceManager;
pub use oauth2::{AccessToken, AuthorizationCode, OAuth2Client, OAuth2Manager, OAuth2TokenInfo};
pub use online::{
    InMemoryPusher, MessagePusher, MessageType, OnlineManager, OnlineUser, PushMessage,
};
pub use permission::{PermissionChecker, RoleChecker};
pub use refresh::RefreshTokenManager;
pub use router::{
    AuthFlowResult, PathAuthConfig, extract_token, match_any, match_path, need_auth, run_auth_flow,
};
pub use safe::{DEFAULT_SAFE_SERVICE, SAFE_AUTH_VALUE};
pub use same_token::{
    DEFAULT_TEMP_NAMESPACE, SAME_TOKEN_HEADER, SameTokenTemplate, TempTokenTemplate,
};
pub use session::SaSession;
pub use session::SaTerminalInfo;
pub use sso::{
    CheckTicketResult, SsoClient, SsoConfig, SsoManager, SsoServer, SsoSession, SsoTicket,
};
pub use stp_logic::SaLogic;
pub use token::{JwtAlgorithm, JwtClaims, JwtManager, TokenInfo, TokenValue};
pub use ws::{DefaultWsTokenExtractor, WsAuthInfo, WsAuthManager, WsTokenExtractor};
