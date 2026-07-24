// Author: 金书记
//
//! 请求/响应上下文适配器trait定义

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 请求上下文trait
///
/// 各个Web框架需要为其Request类型实现这个trait
pub trait SaRequest {
    /// 获取请求头
    fn get_header(&self, name: &str) -> Option<String>;

    /// 获取所有请求头
    fn get_headers(&self) -> HashMap<String, String> {
        HashMap::new() // 默认实现
    }

    /// 获取Cookie
    fn get_cookie(&self, name: &str) -> Option<String>;

    /// 获取所有Cookie
    fn get_cookies(&self) -> HashMap<String, String> {
        HashMap::new() // 默认实现
    }

    /// 获取查询参数
    fn get_param(&self, name: &str) -> Option<String>;

    /// 获取所有查询参数
    fn get_params(&self) -> HashMap<String, String> {
        HashMap::new() // 默认实现
    }

    /// 获取请求路径
    fn get_path(&self) -> String;

    /// 获取请求方法
    fn get_method(&self) -> String;

    /// 获取请求URI
    fn get_uri(&self) -> String {
        self.get_path()
    }

    /// 获取请求体（如果是JSON）
    fn get_body_json<T: for<'de> Deserialize<'de>>(&self) -> Option<T> {
        None // 默认实现
    }

    /// 获取客户端IP
    fn get_client_ip(&self) -> Option<String> {
        None // 默认实现
    }

    /// 获取User-Agent
    fn get_user_agent(&self) -> Option<String> {
        self.get_header("user-agent")
    }
}

/// 响应上下文trait
///
/// 各个Web框架需要为其Response类型实现这个trait
pub trait SaResponse {
    /// 设置响应头
    fn set_header(&mut self, name: &str, value: &str);

    /// 设置Cookie
    fn set_cookie(&mut self, name: &str, value: &str, options: CookieOptions);

    /// Set a cookie using an explicit environment security policy.
    fn set_cookie_with_policy(&mut self, name: &str, value: &str, policy: CookieSecurityPolicy) {
        self.set_cookie(name, value, CookieOptions::for_policy(policy));
    }

    /// 删除Cookie
    fn delete_cookie(&mut self, name: &str) {
        self.set_cookie(
            name,
            "",
            CookieOptions {
                max_age: Some(0),
                ..Default::default()
            },
        );
    }

    /// 设置状态码
    fn set_status(&mut self, status: u16);

    /// 设置响应体（JSON）
    fn set_json_body<T: Serialize>(&mut self, body: T) -> Result<(), serde_json::Error>;
}

/// Cookie deployment security policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CookieSecurityPolicy {
    /// HTTPS production deployment: Secure + HttpOnly + SameSite=Lax.
    ProductionHttps,
    /// Local HTTP development: HttpOnly + SameSite=Lax without Secure.
    LocalDevelopment,
    /// Preserve Java Sa-Token's historical unset/false cookie defaults.
    JavaCompatibility,
}

/// Cookie option validation error.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CookieOptionsError {
    #[error("SameSite=None cookies must also set Secure")]
    SameSiteNoneRequiresSecure,
}

/// Cookie 选项
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CookieOptions {
    /// 域名
    pub domain: Option<String>,

    /// 路径
    pub path: Option<String>,

    /// 过期时间（秒）
    pub max_age: Option<i64>,

    /// 是否仅HTTP
    pub http_only: bool,

    /// 是否安全（仅HTTPS）
    pub secure: bool,

    /// SameSite属性
    pub same_site: Option<SameSite>,
}

impl CookieOptions {
    /// Build cookie defaults for an explicit deployment policy.
    pub fn for_policy(policy: CookieSecurityPolicy) -> Self {
        match policy {
            CookieSecurityPolicy::ProductionHttps => Self {
                domain: None,
                path: Some("/".to_string()),
                max_age: None,
                http_only: true,
                secure: true,
                same_site: Some(SameSite::Lax),
            },
            CookieSecurityPolicy::LocalDevelopment => Self {
                domain: None,
                path: Some("/".to_string()),
                max_age: None,
                http_only: true,
                secure: false,
                same_site: Some(SameSite::Lax),
            },
            CookieSecurityPolicy::JavaCompatibility => Self {
                domain: None,
                path: None,
                max_age: None,
                http_only: false,
                secure: false,
                same_site: None,
            },
        }
    }

    /// Validate browser-enforced cookie invariants.
    pub fn validate(&self) -> Result<(), CookieOptionsError> {
        if self.same_site == Some(SameSite::None) && !self.secure {
            return Err(CookieOptionsError::SameSiteNoneRequiresSecure);
        }
        Ok(())
    }

    /// Explicit local-development defaults for HTTP servers.
    pub fn local_development() -> Self {
        Self::for_policy(CookieSecurityPolicy::LocalDevelopment)
    }

    /// Historical Java-compatible defaults.
    pub fn java_compatibility() -> Self {
        Self::for_policy(CookieSecurityPolicy::JavaCompatibility)
    }
}

impl Default for CookieOptions {
    fn default() -> Self {
        Self::for_policy(CookieSecurityPolicy::ProductionHttps)
    }
}

/// SameSite 属性
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SameSite {
    Strict,
    Lax,
    None,
}

impl std::fmt::Display for SameSite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SameSite::Strict => write!(f, "Strict"),
            SameSite::Lax => write!(f, "Lax"),
            SameSite::None => write!(f, "None"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_cookie_policy_is_secure_for_production() {
        let options = CookieOptions::default();

        assert_eq!(options.path.as_deref(), Some("/"));
        assert!(options.http_only);
        assert!(options.secure);
        assert_eq!(options.same_site, Some(SameSite::Lax));
        assert!(options.validate().is_ok());
    }

    #[test]
    fn local_development_policy_keeps_http_only_without_secure() {
        let options = CookieOptions::local_development();

        assert!(options.http_only);
        assert!(!options.secure);
        assert_eq!(options.same_site, Some(SameSite::Lax));
        assert!(options.validate().is_ok());
    }

    #[test]
    fn same_site_none_requires_secure() {
        let options = CookieOptions {
            same_site: Some(SameSite::None),
            secure: false,
            ..CookieOptions::default()
        };

        assert_eq!(
            options.validate(),
            Err(CookieOptionsError::SameSiteNoneRequiresSecure)
        );
    }

    #[test]
    fn java_compatibility_policy_is_explicit() {
        let options = CookieOptions::java_compatibility();

        assert!(!options.http_only);
        assert!(!options.secure);
        assert_eq!(options.same_site, None);
    }
}
