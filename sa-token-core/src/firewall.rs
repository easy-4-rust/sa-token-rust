// Author: 金书记
//
//! Firewall strategy module | 防火墙策略模块
//!
//! Provides request-level security checks: path whitelist/blacklist,
//! danger character detection, directory traversal prevention,
//! host/method/header/parameter validation.
//!
//! 对应 Java `cn.dev33.satoken.strategy.SaFirewallStrategy` + 10 个 Hook。
//!
//! ## Design
//!
//! Java uses a `SaFirewallCheckHook` interface with 10 separate classes.
//! Rust consolidates into a single module with an enum-based hook registry,
//! keeping the same semantics in ~400 lines instead of 14 files.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use sa_token_core::firewall::FirewallStrategy;
//!
//! let mut fw = FirewallStrategy::default();
//! fw.add_white_path("/login");
//! fw.add_black_path("/admin/secret");
//!
//! // In middleware:
//! fw.check(&request, &response)?;
//! ```

use std::sync::RwLock;

use sa_token_adapter::context::SaRequest;

use crate::error::SaTokenError;

// ── Default danger characters (Java SaFirewallCheckHookForPathDangerCharacter) ──

/// Default danger characters that should never appear in a request path.
/// 对应 Java `SaFirewallCheckHookForPathDangerCharacter.dangerCharacter`。
pub const DEFAULT_DANGER_CHARACTERS: &[&str] = &[
    "//", "\\", "%2e", "%2E", "%2f", "%2F", "%5c", "%5C", ";", "%3b", "%3B", "%25", "\0", "%00",
    "\n", "%0a", "%0A", "\r", "%0d", "%0D", "\u{2028}", "\u{2029}",
];

/// Default allowed HTTP methods.
/// 对应 Java `SaFirewallCheckHookForHttpMethod.allowMethods`。
pub const DEFAULT_ALLOW_METHODS: &[&str] = &[
    "GET", "POST", "PUT", "DELETE", "HEAD", "OPTIONS", "PATCH", "TRACE", "CONNECT",
];

// ── Firewall Check Result ──

/// Internal control flow: `StopMatch` means the request is whitelisted
/// and remaining hooks should be skipped (Java `StopMatchException`).
#[derive(PartialEq)]
enum CheckControl {
    /// Continue to next hook
    Continue,
    /// Request is whitelisted; skip remaining hooks
    StopMatch,
}

// ── Path validation utilities ──

/// Check if a path is valid (no directory traversal).
/// 对应 Java `SaFirewallCheckHookForDirectoryTraversal.isPathValid`。
pub fn is_path_valid(path: &str) -> bool {
    if path.is_empty() {
        return false;
    }
    if !path.starts_with('/') {
        return false;
    }
    if path == "/" {
        return true;
    }
    let components: Vec<&str> = path.split('/').collect();
    for (i, component) in components.iter().enumerate() {
        if component.is_empty() {
            if i == 0 {
                continue; // allow leading "/"
            } else {
                return false; // "//" in middle or end
            }
        }
        if *component == "." || *component == ".." {
            return false;
        }
    }
    true
}

/// Check if a string contains any non-printable ASCII character.
/// 对应 Java `SaFoxUtil.hasNonPrintableASCII`。
pub fn has_non_printable_ascii(s: &str) -> bool {
    s.chars().any(|c| {
        let code = c as u32;
        code < 0x20 || code == 0x7f
    })
}

// ── Firewall Strategy ──

/// Firewall strategy configuration and execution.
///
/// 对应 Java `SaFirewallStrategy.instance`。
/// All hooks are checked in order; the first failing hook aborts with an error.
#[derive(Clone)]
pub struct FirewallStrategy {
    // Path whitelist (checked first; match = skip remaining hooks)
    white_paths: Vec<String>,
    // Path blacklist
    black_paths: Vec<String>,
    // Danger characters in path
    danger_characters: Vec<String>,
    // Whether to ban '%' in paths
    banned_percentage: bool,
    // Whether to ban non-printable ASCII in paths
    banned_non_printable: bool,
    // Host check
    check_host: bool,
    allow_hosts: Vec<String>,
    // HTTP method check
    check_method: bool,
    allow_methods: Vec<String>,
    // Header blacklist
    not_allow_header_names: Vec<String>,
    // Parameter blacklist
    not_allow_parameter_names: Vec<String>,
}

impl Default for FirewallStrategy {
    fn default() -> Self {
        Self {
            white_paths: Vec::new(),
            black_paths: Vec::new(),
            danger_characters: DEFAULT_DANGER_CHARACTERS
                .iter()
                .map(|s| s.to_string())
                .collect(),
            banned_percentage: false,
            banned_non_printable: true,
            check_host: false,
            allow_hosts: Vec::new(),
            check_method: true,
            allow_methods: DEFAULT_ALLOW_METHODS
                .iter()
                .map(|s| s.to_string())
                .collect(),
            not_allow_header_names: Vec::new(),
            not_allow_parameter_names: Vec::new(),
        }
    }
}

impl FirewallStrategy {
    /// Create a new default firewall strategy
    pub fn new() -> Self {
        Self::default()
    }

    // ── Configuration builders ──

    /// Add a path to the whitelist (Java `whitePaths.add(...)`)
    pub fn add_white_path(&mut self, path: impl Into<String>) -> &mut Self {
        self.white_paths.push(path.into());
        self
    }

    /// Add a path to the blacklist (Java `blackPaths.add(...)`)
    pub fn add_black_path(&mut self, path: impl Into<String>) -> &mut Self {
        self.black_paths.push(path.into());
        self
    }

    /// Set the danger characters list (Java `dangerCharacter = ...`)
    pub fn set_danger_characters(&mut self, chars: Vec<String>) -> &mut Self {
        self.danger_characters = chars;
        self
    }

    /// Enable or disable banning '%' in paths (Java `bannedPercentage`)
    pub fn set_banned_percentage(&mut self, banned: bool) -> &mut Self {
        self.banned_percentage = banned;
        self
    }

    /// Enable or disable host checking (Java `isCheckHost`)
    pub fn set_check_host(&mut self, check: bool) -> &mut Self {
        self.check_host = check;
        self
    }

    /// Add an allowed host (Java `allowHosts.add(...)`)
    pub fn add_allow_host(&mut self, host: impl Into<String>) -> &mut Self {
        self.allow_hosts.push(host.into());
        self
    }

    /// Enable or disable HTTP method checking (Java `isCheckMethod`)
    pub fn set_check_method(&mut self, check: bool) -> &mut Self {
        self.check_method = check;
        self
    }

    /// Add a disallowed header name (Java `notAllowHeaderNames.add(...)`)
    pub fn add_not_allow_header(&mut self, name: impl Into<String>) -> &mut Self {
        self.not_allow_header_names.push(name.into());
        self
    }

    /// Add a disallowed parameter name (Java `notAllowParameterNames.add(...)`)
    pub fn add_not_allow_parameter(&mut self, name: impl Into<String>) -> &mut Self {
        self.not_allow_parameter_names.push(name.into());
        self
    }

    // ── Execution ──

    /// Run all firewall checks against a request.
    ///
    /// 对应 Java `SaFirewallStrategy.instance.check.execute(req, res, extArg)`。
    ///
    /// # Errors
    ///
    /// Returns `SaTokenError::FirewallCheck` or `SaTokenError::RequestPathInvalid`
    /// when a check fails.
    /// Run all firewall checks against a request.
    ///
    /// 对应 Java `SaFirewallStrategy.instance.check.execute(req, res, extArg)`。
    ///
    /// # Errors
    ///
    /// Returns `SaTokenError::FirewallCheck` or `SaTokenError::RequestPathInvalid`
    /// when a check fails.
    pub fn check<R: SaRequest>(&self, req: &R) -> Result<(), SaTokenError> {
        let path = req.get_path();

        // 1. WhitePath: if path is whitelisted, skip all remaining hooks
        if self.check_white_path(&path)? == CheckControl::StopMatch {
            return Ok(());
        }

        // 2. BlackPath
        self.check_black_path(&path)?;

        // 3. Path danger characters
        self.check_danger_characters(&path)?;

        // 4. Path banned characters (non-printable ASCII + optional %)
        self.check_banned_characters(&path)?;

        // 5. Directory traversal
        self.check_directory_traversal(&path)?;

        // 6. Host (via Host header)
        self.check_host(req)?;

        // 7. HTTP Method
        self.check_method(req)?;

        // 8. Header
        self.check_header(req)?;

        // 9. Parameter
        self.check_parameter(req)?;

        Ok(())
    }

    // ── Individual hook implementations ──

    fn check_white_path(&self, path: &str) -> Result<CheckControl, SaTokenError> {
        for white in &self.white_paths {
            if path == white {
                return Ok(CheckControl::StopMatch);
            }
        }
        Ok(CheckControl::Continue)
    }

    fn check_black_path(&self, path: &str) -> Result<(), SaTokenError> {
        for black in &self.black_paths {
            if path == black {
                return Err(SaTokenError::RequestPathInvalid {
                    path: path.to_string(),
                    message: format!("Illegal request path (blacklisted): {path}"),
                });
            }
        }
        Ok(())
    }

    fn check_danger_characters(&self, path: &str) -> Result<(), SaTokenError> {
        for danger in &self.danger_characters {
            if path.contains(danger.as_str()) {
                return Err(SaTokenError::RequestPathInvalid {
                    path: path.to_string(),
                    message: format!("Illegal request path (danger character '{danger}'): {path}"),
                });
            }
        }
        Ok(())
    }

    fn check_banned_characters(&self, path: &str) -> Result<(), SaTokenError> {
        if self.banned_non_printable && has_non_printable_ascii(path) {
            return Err(SaTokenError::RequestPathInvalid {
                path: path.to_string(),
                message: format!("Request path contains banned non-printable characters: {path}"),
            });
        }
        if self.banned_percentage && path.contains('%') {
            return Err(SaTokenError::RequestPathInvalid {
                path: path.to_string(),
                message: format!("Request path contains banned '%' character: {path}"),
            });
        }
        Ok(())
    }

    fn check_directory_traversal(&self, path: &str) -> Result<(), SaTokenError> {
        if !is_path_valid(path) {
            return Err(SaTokenError::RequestPathInvalid {
                path: path.to_string(),
                message: format!("Illegal request path (directory traversal): {path}"),
            });
        }
        Ok(())
    }

    fn check_host<R: SaRequest>(&self, req: &R) -> Result<(), SaTokenError> {
        if !self.check_host {
            return Ok(());
        }
        let host = req.get_header("host").unwrap_or_default();
        let allowed = self.allow_hosts.iter().any(|h| {
            if let Some(suffix) = h.strip_prefix("*") {
                host.ends_with(suffix)
            } else {
                host == *h
            }
        });
        if !allowed {
            return Err(SaTokenError::FirewallCheck {
                message: format!("Illegal request host: {host}"),
            });
        }
        Ok(())
    }

    fn check_method<R: SaRequest>(&self, req: &R) -> Result<(), SaTokenError> {
        if !self.check_method {
            return Ok(());
        }
        let method = req.get_method();
        let allowed = self
            .allow_methods
            .iter()
            .any(|m| m.eq_ignore_ascii_case(&method));
        if !allowed {
            return Err(SaTokenError::FirewallCheck {
                message: format!("Illegal request method: {method}"),
            });
        }
        Ok(())
    }

    fn check_header<R: SaRequest>(&self, req: &R) -> Result<(), SaTokenError> {
        for name in &self.not_allow_header_names {
            if req.get_header(name).is_some() {
                return Err(SaTokenError::FirewallCheck {
                    message: format!("Illegal request header: {name}"),
                });
            }
        }
        Ok(())
    }

    fn check_parameter<R: SaRequest>(&self, req: &R) -> Result<(), SaTokenError> {
        for name in &self.not_allow_parameter_names {
            if req.get_param(name).is_some() {
                return Err(SaTokenError::FirewallCheck {
                    message: format!("Illegal request parameter: {name}"),
                });
            }
        }
        Ok(())
    }
}

/// Thread-safe wrapper for global firewall strategy (Java singleton pattern).
/// 对应 Java `SaFirewallStrategy.instance`。
pub struct GlobalFirewall;

static GLOBAL_FIREWALL: RwLock<Option<FirewallStrategy>> = RwLock::new(None);

impl GlobalFirewall {
    /// Get the global firewall strategy, initializing with defaults if not set.
    pub fn get() -> FirewallStrategy {
        let guard = GLOBAL_FIREWALL.read().unwrap_or_else(|e| e.into_inner());
        guard.clone().unwrap_or_default()
    }

    /// Set the global firewall strategy.
    pub fn set(strategy: FirewallStrategy) {
        let mut guard = GLOBAL_FIREWALL.write().unwrap_or_else(|e| e.into_inner());
        *guard = Some(strategy);
    }

    /// Reset to default.
    pub fn reset() {
        let mut guard = GLOBAL_FIREWALL.write().unwrap_or_else(|e| e.into_inner());
        *guard = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_path_valid() {
        assert!(is_path_valid("/user/info"));
        assert!(is_path_valid("/"));
        assert!(is_path_valid("/user/info/.js"));
        assert!(!is_path_valid("/user/info/."));
        assert!(!is_path_valid("/user/info/.."));
        assert!(!is_path_valid("/user/./info"));
        assert!(!is_path_valid("/user/../info"));
        assert!(!is_path_valid("//user"));
        assert!(!is_path_valid("/user//info"));
        assert!(!is_path_valid(""));
        assert!(!is_path_valid("user/info"));
    }

    #[test]
    fn test_has_non_printable_ascii() {
        assert!(!has_non_printable_ascii("/api/users"));
        assert!(has_non_printable_ascii("/api\u{0001}users"));
        assert!(has_non_printable_ascii("/api\u{007f}users"));
        assert!(!has_non_printable_ascii("/api/用户")); // Unicode is fine
    }

    #[test]
    fn test_danger_characters_detected() {
        let fw = FirewallStrategy::new();
        // Default danger characters include "//"
        // We test the check logic directly
        assert!(fw.check_danger_characters("/api//users").is_err());
        assert!(fw.check_danger_characters("/api/users").is_ok());
        assert!(fw.check_danger_characters("/api\\users").is_err());
    }

    #[test]
    fn test_white_path_skips_remaining() {
        let mut fw = FirewallStrategy::new();
        fw.add_white_path("/login");
        fw.add_black_path("/login"); // would normally fail
        // White path takes priority
        let control = fw.check_white_path("/login").unwrap();
        assert!(matches!(control, CheckControl::StopMatch));
    }

    #[test]
    fn test_black_path_rejected() {
        let mut fw = FirewallStrategy::new();
        fw.add_black_path("/admin/secret");
        assert!(fw.check_black_path("/admin/secret").is_err());
        assert!(fw.check_black_path("/admin/public").is_ok());
    }

    #[test]
    fn test_directory_traversal_rejected() {
        let fw = FirewallStrategy::new();
        assert!(fw.check_directory_traversal("/../etc/passwd").is_err());
        assert!(fw.check_directory_traversal("/api/users").is_ok());
    }

    #[test]
    fn test_global_firewall_round_trip() {
        GlobalFirewall::reset();
        let default = GlobalFirewall::get();
        assert!(default.white_paths.is_empty());
        let mut custom = FirewallStrategy::new();
        custom.add_white_path("/health");
        GlobalFirewall::set(custom);
        let got = GlobalFirewall::get();
        assert_eq!(got.white_paths, vec!["/health"]);
        GlobalFirewall::reset();
    }

    #[test]
    fn test_default_danger_characters() {
        assert!(DEFAULT_DANGER_CHARACTERS.contains(&"//"));
        assert!(DEFAULT_DANGER_CHARACTERS.contains(&"\\"));
        assert!(DEFAULT_DANGER_CHARACTERS.contains(&"%00"));
    }

    #[test]
    fn test_default_allow_methods() {
        assert!(DEFAULT_ALLOW_METHODS.contains(&"GET"));
        assert!(DEFAULT_ALLOW_METHODS.contains(&"POST"));
        assert!(!DEFAULT_ALLOW_METHODS.contains(&"PROPFIND"));
    }
}
