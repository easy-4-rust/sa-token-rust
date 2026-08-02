// Author: 金书记
//
//! HTTP Filter trait | HTTP 过滤器接口
//!
//! 对应 Java `cn.dev33.satoken.filter.SaFilter`。
//!
//! 为不同 Web 框架的过滤器/中间件提供统一行为接口。
//! Web 框架适配层（axum/actix/poem 等）应实现此 trait。

use std::fmt;
use std::sync::Arc;

use crate::error::SaTokenError;
use crate::runtime::SaTokenRuntime;

/// 认证函数类型
pub type AuthFn = Arc<dyn Fn(&SaTokenRuntime) -> Result<(), SaTokenError> + Send + Sync>;

/// 异常处理函数类型
pub type ErrorFn = Arc<dyn Fn(SaTokenError) + Send + Sync>;

/// 前置函数类型（不受 include/exclude 限制）
pub type BeforeAuthFn = Arc<dyn Fn() + Send + Sync>;

/// HTTP Filter trait
///
/// 对应 Java `SaFilter` 接口。
/// 定义了认证过滤器的统一行为，包括：
/// - 路由拦截/放行配置
/// - 认证函数
/// - 异常处理函数
/// - 前置/后置钩子
pub trait SaFilter: Send + Sync + 'static {
    /// 添加拦截路由（对应 Java `addInclude`）
    fn add_include(&mut self, paths: &[&str]);

    /// 添加放行路由（对应 Java `addExclude`）
    fn add_exclude(&mut self, paths: &[&str]);

    /// 设置拦截路由列表（对应 Java `setIncludeList`）
    fn set_include_list(&mut self, paths: Vec<String>);

    /// 设置放行路由列表（对应 Java `setExcludeList`）
    fn set_exclude_list(&mut self, paths: Vec<String>);

    /// 获取拦截路由列表
    fn include_list(&self) -> &[String];

    /// 获取放行路由列表
    fn exclude_list(&self) -> &[String];

    /// 设置认证函数（对应 Java `setAuth`）
    fn set_auth(&mut self, auth: AuthFn);

    /// 设置异常处理函数（对应 Java `setError`）
    fn set_error(&mut self, error: ErrorFn);

    /// 设置前置函数（对应 Java `setBeforeAuth`）
    fn set_before_auth(&mut self, before: BeforeAuthFn);

    /// 执行认证检查
    fn do_auth(&self, runtime: &SaTokenRuntime) -> Result<(), SaTokenError>;

    /// 执行异常处理
    fn do_error(&self, error: SaTokenError);

    /// 执行前置函数
    fn do_before_auth(&self);
}

/// Filter 配置构建器
#[derive(Clone, Default)]
pub struct SaFilterConfig {
    /// 拦截路由
    pub include_list: Vec<String>,
    /// 放行路由
    pub exclude_list: Vec<String>,
    /// 认证函数
    pub auth_fn: Option<AuthFn>,
    /// 异常处理函数
    pub error_fn: Option<ErrorFn>,
    /// 前置函数
    pub before_auth_fn: Option<BeforeAuthFn>,
}

impl fmt::Debug for SaFilterConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SaFilterConfig")
            .field("include_list", &self.include_list)
            .field("exclude_list", &self.exclude_list)
            .field("has_auth_fn", &self.auth_fn.is_some())
            .field("has_error_fn", &self.error_fn.is_some())
            .field("has_before_auth_fn", &self.before_auth_fn.is_some())
            .finish()
    }
}

impl SaFilterConfig {
    /// 创建新的配置
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加拦截路由
    pub fn include(mut self, paths: &[&str]) -> Self {
        self.include_list
            .extend(paths.iter().map(|s| s.to_string()));
        self
    }

    /// 添加放行路由
    pub fn exclude(mut self, paths: &[&str]) -> Self {
        self.exclude_list
            .extend(paths.iter().map(|s| s.to_string()));
        self
    }

    /// 设置认证函数
    pub fn auth<F>(mut self, f: F) -> Self
    where
        F: Fn(&SaTokenRuntime) -> Result<(), SaTokenError> + Send + Sync + 'static,
    {
        self.auth_fn = Some(Arc::new(f));
        self
    }

    /// 设置异常处理函数
    pub fn error<F>(mut self, f: F) -> Self
    where
        F: Fn(SaTokenError) + Send + Sync + 'static,
    {
        self.error_fn = Some(Arc::new(f));
        self
    }

    /// 设置前置函数
    pub fn before_auth<F>(mut self, f: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.before_auth_fn = Some(Arc::new(f));
        self
    }

    /// 检查路径是否应该被拦截
    pub fn should_intercept(&self, path: &str) -> bool {
        // 如果 include_list 为空，默认拦截所有
        if self.include_list.is_empty() {
            return true;
        }

        // 检查是否在拦截列表中
        let included = self.include_list.iter().any(|p| path_matches(path, p));

        // 检查是否在放行列表中
        let excluded = self.exclude_list.iter().any(|p| path_matches(path, p));

        included && !excluded
    }
}

/// 路径匹配（支持通配符 * 和 **）
fn path_matches(path: &str, pattern: &str) -> bool {
    // 简单的通配符匹配实现
    if pattern == "*" || pattern == "**" {
        return true;
    }

    if pattern.ends_with("/**") {
        let prefix = pattern.strip_suffix("/**").unwrap_or(pattern);
        return path.starts_with(prefix);
    }

    if pattern.ends_with("/*") {
        let prefix = pattern.strip_suffix("/*").unwrap_or(pattern);
        if !path.starts_with(prefix) {
            return false;
        }
        let remaining = &path[prefix.len()..];
        // 去掉开头的 /，然后检查是否还有其他 /
        let remaining = remaining.strip_prefix('/').unwrap_or(remaining);
        return remaining.is_empty() || !remaining.contains('/');
    }

    if pattern.starts_with("*.") {
        let suffix = pattern.strip_prefix("*").unwrap_or(pattern);
        return path.ends_with(suffix);
    }

    path == pattern
}

/// 默认的 Filter 实现
#[derive(Debug, Clone)]
pub struct DefaultSaFilter {
    config: SaFilterConfig,
}

impl DefaultSaFilter {
    pub fn new(config: SaFilterConfig) -> Self {
        Self { config }
    }
}

impl SaFilter for DefaultSaFilter {
    fn add_include(&mut self, paths: &[&str]) {
        self.config
            .include_list
            .extend(paths.iter().map(|s| s.to_string()));
    }

    fn add_exclude(&mut self, paths: &[&str]) {
        self.config
            .exclude_list
            .extend(paths.iter().map(|s| s.to_string()));
    }

    fn set_include_list(&mut self, paths: Vec<String>) {
        self.config.include_list = paths;
    }

    fn set_exclude_list(&mut self, paths: Vec<String>) {
        self.config.exclude_list = paths;
    }

    fn include_list(&self) -> &[String] {
        &self.config.include_list
    }

    fn exclude_list(&self) -> &[String] {
        &self.config.exclude_list
    }

    fn set_auth(&mut self, auth: AuthFn) {
        self.config.auth_fn = Some(auth);
    }

    fn set_error(&mut self, error: ErrorFn) {
        self.config.error_fn = Some(error);
    }

    fn set_before_auth(&mut self, before: BeforeAuthFn) {
        self.config.before_auth_fn = Some(before);
    }

    fn do_auth(&self, runtime: &SaTokenRuntime) -> Result<(), SaTokenError> {
        if let Some(ref auth_fn) = self.config.auth_fn {
            auth_fn(runtime)
        } else {
            Ok(())
        }
    }

    fn do_error(&self, error: SaTokenError) {
        if let Some(ref error_fn) = self.config.error_fn {
            error_fn(error);
        }
    }

    fn do_before_auth(&self) {
        if let Some(ref before_fn) = self.config.before_auth_fn {
            before_fn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_matches_wildcard() {
        assert!(path_matches("/api/users", "/api/**"));
        assert!(path_matches("/api/users/123", "/api/**"));
        assert!(!path_matches("/other/path", "/api/**"));
    }

    #[test]
    fn test_path_matches_single_wildcard() {
        assert!(path_matches("/api/users", "/api/*"));
        assert!(!path_matches("/api/users/123", "/api/*"));
    }

    #[test]
    fn test_path_matches_extension() {
        assert!(path_matches("/page.html", "*.html"));
        assert!(!path_matches("/page.css", "*.html"));
    }

    #[test]
    fn test_path_matches_exact() {
        assert!(path_matches("/api/users", "/api/users"));
        assert!(!path_matches("/api/users/123", "/api/users"));
    }

    #[test]
    fn test_should_intercept_default_all() {
        let config = SaFilterConfig::new();
        assert!(config.should_intercept("/any/path"));
    }

    #[test]
    fn test_should_intercept_include_exclude() {
        let config = SaFilterConfig::new()
            .include(&["/api/**"])
            .exclude(&["/api/public/**"]);

        assert!(config.should_intercept("/api/users"));
        assert!(!config.should_intercept("/api/public/health"));
        assert!(!config.should_intercept("/other/path"));
    }

    #[test]
    fn test_filter_config_builder() {
        let config = SaFilterConfig::new()
            .include(&["/api/**"])
            .exclude(&["/api/public/**"])
            .auth(|_runtime| Ok(()));

        assert_eq!(config.include_list.len(), 1);
        assert_eq!(config.exclude_list.len(), 1);
        assert!(config.auth_fn.is_some());
    }
}
