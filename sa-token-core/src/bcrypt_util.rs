// Author: 金书记
//
//! BCrypt password hashing utilities | BCrypt 密码哈希工具
//!
//! 对应 Java `cn.dev33.satoken.secure.BCrypt`。
//!
//! Java 版本已废弃（使用 jBCrypt），Rust 版本使用 `bcrypt` crate，
//! 提供更安全、更现代的实现。

use crate::error::SaTokenError;

/// BCrypt 工具结构体
pub struct BCryptUtil;

impl BCryptUtil {
    /// 哈希密码（对应 Java `BCrypt.hashpw(password, salt)`）
    ///
    /// # Arguments
    /// * `password` - 明文密码
    /// * `cost` - 加密强度（4-31，默认 12）
    ///
    /// # Returns
    /// 哈希后的密码字符串
    pub fn hash(password: &str, cost: u32) -> Result<String, SaTokenError> {
        bcrypt::hash(password, cost)
            .map_err(|e| SaTokenError::InternalError(format!("BCrypt hash failed: {}", e)))
    }

    /// 使用默认强度（12）哈希密码
    pub fn hash_default(password: &str) -> Result<String, SaTokenError> {
        Self::hash(password, 12)
    }

    /// 验证密码（对应 Java `BCrypt.checkpw(password, hash)`）
    ///
    /// # Arguments
    /// * `password` - 明文密码
    /// * `hash` - 哈希后的密码
    ///
    /// # Returns
    /// 密码是否匹配
    pub fn verify(password: &str, hash: &str) -> Result<bool, SaTokenError> {
        bcrypt::verify(password, hash)
            .map_err(|e| SaTokenError::InternalError(format!("BCrypt verify failed: {}", e)))
    }

    /// 生成随机盐值并返回哈希结果（对应 Java `BCrypt.gensalt()`）
    pub fn hash_with_generated_salt(password: &str, cost: u32) -> Result<String, SaTokenError> {
        bcrypt::hash(password, cost)
            .map_err(|e| SaTokenError::InternalError(format!("BCrypt hash failed: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_and_verify() {
        let password = "my_secure_password";
        let hash = BCryptUtil::hash_default(password).unwrap();
        assert!(BCryptUtil::verify(password, &hash).unwrap());
        assert!(!BCryptUtil::verify("wrong_password", &hash).unwrap());
    }

    #[test]
    fn test_hash_with_custom_cost() {
        let password = "test_password";
        let hash = BCryptUtil::hash(password, 6).unwrap(); // 低 cost 用于测试速度
        assert!(BCryptUtil::verify(password, &hash).unwrap());
    }

    #[test]
    fn test_hash_with_generated_salt() {
        let password = "another_password";
        let hash1 = BCryptUtil::hash_with_generated_salt(password, 6).unwrap();
        let hash2 = BCryptUtil::hash_with_generated_salt(password, 6).unwrap();
        // 两次哈希应该不同（因为盐值随机）
        assert_ne!(hash1, hash2);
        // 但都能验证通过
        assert!(BCryptUtil::verify(password, &hash1).unwrap());
        assert!(BCryptUtil::verify(password, &hash2).unwrap());
    }
}
