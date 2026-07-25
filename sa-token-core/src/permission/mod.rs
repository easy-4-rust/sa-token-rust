// Author: 金书记
//
//! 权限验证模块

use crate::error::SaTokenResult;
use async_trait::async_trait;

/// 权限检查器 | Permission Checker
///
/// 用于检查用户权限的 trait
/// Trait for checking user permissions
///
/// # 使用示例 | Usage Example
///
/// ```rust,ignore
/// use async_trait::async_trait;
/// use sa_token_core::PermissionChecker;
///
/// struct MyPermissionChecker;
///
/// #[async_trait]
/// impl PermissionChecker for MyPermissionChecker {
///     async fn has_permission(&self, login_id: &str, permission: &str) -> SaTokenResult<bool> {
///         // 从数据库查询权限 | Query permission from database
///         Ok(true)
///     }
///     
///     async fn get_permissions(&self, login_id: &str) -> SaTokenResult<Vec<String>> {
///         // 返回用户所有权限 | Return all user permissions
///         Ok(vec!["read".to_string(), "write".to_string()])
///     }
/// }
/// ```
#[async_trait]
pub trait PermissionChecker: Send + Sync {
    /// 检查用户是否拥有指定权限 | Check if User Has Specific Permission
    ///
    /// # 参数 | Parameters
    /// - `login_id`: 登录 ID | Login ID
    /// - `permission`: 权限标识（如 "user:read", "admin:*"）| Permission identifier (e.g., "user:read", "admin:*")
    ///
    /// # 返回 | Returns
    /// - `Ok(true)`: 用户拥有该权限 | User has the permission
    /// - `Ok(false)`: 用户没有该权限 | User doesn't have the permission
    async fn has_permission(&self, login_id: &str, permission: &str) -> SaTokenResult<bool>;

    /// 检查用户是否拥有所有指定权限（AND 逻辑）
    /// Check if User Has All Specified Permissions (AND logic)
    ///
    /// # 参数 | Parameters
    /// - `login_id`: 登录 ID | Login ID
    /// - `permissions`: 权限列表 | Permission list
    ///
    /// # 返回 | Returns
    /// 只有当用户拥有所有权限时才返回 true
    /// Returns true only when user has all permissions
    async fn has_all_permissions(
        &self,
        login_id: &str,
        permissions: &[&str],
    ) -> SaTokenResult<bool> {
        for permission in permissions {
            if !self.has_permission(login_id, permission).await? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// 检查用户是否拥有任一指定权限（OR 逻辑）
    /// Check if User Has Any Specified Permission (OR logic)
    ///
    /// # 参数 | Parameters
    /// - `login_id`: 登录 ID | Login ID
    /// - `permissions`: 权限列表 | Permission list
    ///
    /// # 返回 | Returns
    /// 只要用户拥有任一权限就返回 true
    /// ｜ Returns true if user has any of the permissions
    async fn has_any_permission(
        &self,
        login_id: &str,
        permissions: &[&str],
    ) -> SaTokenResult<bool> {
        for permission in permissions {
            if self.has_permission(login_id, permission).await? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// 获取用户的所有权限列表 | Get All User Permissions
    ///
    /// # 参数 | Parameters
    /// - `login_id`: 登录 ID | Login ID
    ///
    /// # 返回 | Returns
    /// 用户的权限列表 | User's permission list
    async fn get_permissions(&self, login_id: &str) -> SaTokenResult<Vec<String>>;
}

/// 角色检查器 | Role Checker
///
/// 用于检查用户角色的 trait
/// ｜ Trait for checking user roles
#[async_trait]
pub trait RoleChecker: Send + Sync {
    /// 检查用户是否拥有指定角色 | Check if User Has Specific Role
    ///
    /// # 参数 | Parameters
    /// - `login_id`: 登录 ID | Login ID
    /// - `role`: 角色标识（如 "admin", "vip"）| Role identifier (e.g., "admin", "vip")
    ///
    /// # 返回 | Returns
    /// - `Ok(true)`: 用户拥有该角色 | User has the role
    /// - `Ok(false)`: 用户没有该角色 | User doesn't have the role
    async fn has_role(&self, login_id: &str, role: &str) -> SaTokenResult<bool>;

    /// 检查用户是否拥有所有指定角色（AND 逻辑）
    /// Check if User Has All Specified Roles (AND logic)
    ///
    /// # 参数 | Parameters
    /// - `login_id`: 登录 ID | Login ID
    /// - `roles`: 角色列表 | Role list
    async fn has_all_roles(&self, login_id: &str, roles: &[&str]) -> SaTokenResult<bool> {
        for role in roles {
            if !self.has_role(login_id, role).await? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// 检查用户是否拥有任一指定角色（OR 逻辑）
    /// Check if User Has Any Specified Role (OR logic)
    ///
    /// # 参数 | Parameters
    /// - `login_id`: 登录 ID | Login ID
    /// - `roles`: 角色列表 | Role list
    async fn has_any_role(&self, login_id: &str, roles: &[&str]) -> SaTokenResult<bool> {
        for role in roles {
            if self.has_role(login_id, role).await? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// 获取用户的所有角色列表 | Get All User Roles
    ///
    /// # 参数 | Parameters
    /// - `login_id`: 登录 ID | Login ID
    ///
    /// # 返回 | Returns
    /// 用户的角色列表 | User's role list
    async fn get_roles(&self, login_id: &str) -> SaTokenResult<Vec<String>>;
}
