// Author: 金书记
//
//! 权限服务
//! 
//! 用于管理用户的权限和角色

use std::collections::HashMap;
use tokio::sync::RwLock;

/// 权限服务
pub struct PermissionService {
    /// 用户权限映射 user_id -> permissions
    user_permissions: RwLock<HashMap<String, Vec<String>>>,
    
    /// 用户角色映射 user_id -> roles
    user_roles: RwLock<HashMap<String, Vec<String>>>,
}

impl PermissionService {
    pub fn new() -> Self {
        Self {
            user_permissions: RwLock::new(HashMap::new()),
            user_roles: RwLock::new(HashMap::new()),
        }
    }
    
    /// 添加用户权限
    pub async fn add_user_permissions(&self, user_id: &str, permissions: Vec<String>) {
        let mut map = self.user_permissions.write().await;
        map.insert(user_id.to_string(), permissions);
    }
    
    /// 添加用户角色
    pub async fn add_user_roles(&self, user_id: &str, roles: Vec<String>) {
        let mut map = self.user_roles.write().await;
        map.insert(user_id.to_string(), roles);
    }
    
    /// 获取用户权限列表
    pub async fn get_user_permissions(&self, user_id: &str) -> Vec<String> {
        let map = self.user_permissions.read().await;
        map.get(user_id).cloned().unwrap_or_default()
    }
    
    /// 获取用户角色列表
    pub async fn get_user_roles(&self, user_id: &str) -> Vec<String> {
        let map = self.user_roles.read().await;
        map.get(user_id).cloned().unwrap_or_default()
    }
    
    /// 检查用户是否拥有指定权限
    pub async fn has_permission(&self, user_id: &str, permission: &str) -> bool {
        let map = self.user_permissions.read().await;
        if let Some(permissions) = map.get(user_id) {
            // 检查精确匹配
            if permissions.contains(&permission.to_string()) {
                return true;
            }
            
            // 检查通配符匹配（例如 admin:* 匹配 admin:read）
            for perm in permissions {
                if perm.ends_with(":*") {
                    let prefix = &perm[..perm.len() - 2];
                    if permission.starts_with(prefix) {
                        return true;
                    }
                }
            }
        }
        false
    }
    
    /// 检查用户是否拥有所有指定权限
    pub async fn has_all_permissions(&self, user_id: &str, permissions: &[&str]) -> bool {
        for permission in permissions {
            if !self.has_permission(user_id, permission).await {
                return false;
            }
        }
        true
    }
    
    /// 检查用户是否拥有任一指定权限
    pub async fn has_any_permission(&self, user_id: &str, permissions: &[&str]) -> bool {
        for permission in permissions {
            if self.has_permission(user_id, permission).await {
                return true;
            }
        }
        false
    }
    
    /// 检查用户是否拥有指定角色
    pub async fn has_role(&self, user_id: &str, role: &str) -> bool {
        let map = self.user_roles.read().await;
        if let Some(roles) = map.get(user_id) {
            roles.contains(&role.to_string())
        } else {
            false
        }
    }
    
    /// 检查用户是否拥有所有指定角色
    pub async fn has_all_roles(&self, user_id: &str, roles: &[&str]) -> bool {
        for role in roles {
            if !self.has_role(user_id, role).await {
                return false;
            }
        }
        true
    }
    
    /// 检查用户是否拥有任一指定角色
    pub async fn has_any_role(&self, user_id: &str, roles: &[&str]) -> bool {
        for role in roles {
            if self.has_role(user_id, role).await {
                return true;
            }
        }
        false
    }
    
    /// 从数据库加载用户权限（示例）
    /// 实际应用中应该从数据库加载
    pub async fn load_from_database(&self, _user_id: &str) {
        // TODO: 实现从数据库加载权限和角色
        // 例如：
        // let permissions = database.query_user_permissions(user_id).await?;
        // let roles = database.query_user_roles(user_id).await?;
        // self.add_user_permissions(user_id, permissions).await;
        // self.add_user_roles(user_id, roles).await;
    }
}

impl Default for PermissionService {
    fn default() -> Self {
        Self::new()
    }
}
