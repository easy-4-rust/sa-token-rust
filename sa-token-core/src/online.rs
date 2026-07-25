//! Online User Management and Real-time Push Module | 在线用户管理和实时推送模块
//!
//! # Code Flow Logic | 代码流程逻辑
//!
//! ## English
//!
//! ### Overview
//! This module provides comprehensive online user management and real-time message
//! push capabilities. It tracks user online status, manages connections, and
//! delivers messages to users in real-time.
//!
//! ### Online User Management Flow
//! ```text
//! 1. User Connects (e.g., WebSocket, SSE)
//!    ↓
//! 2. OnlineManager.mark_online(OnlineUser)
//!    ├─→ Store user info in online_users HashMap
//!    ├─→ Key: login_id
//!    └─→ Value: Vec<OnlineUser> (supports multiple devices)
//!    ↓
//! 3. User Activity Updates
//!    OnlineManager.update_activity(login_id, token)
//!    └─→ Update last_activity timestamp
//!    ↓
//! 4. User Disconnects
//!    OnlineManager.mark_offline(login_id, token)
//!    ├─→ Remove from online_users
//!    └─→ Clean up if no more sessions
//! ```
//!
//! ### Message Push Flow
//! ```text
//! 1. Create PushMessage
//!    ├─→ Generate unique message_id
//!    ├─→ Set content and message_type
//!    └─→ Add timestamp and metadata
//!    ↓
//! 2. Select Push Method
//!    ├─→ push_to_user(login_id, content)      - Single user
//!    ├─→ push_to_users(vec![ids], content)    - Multiple users
//!    └─→ broadcast(content)                    - All online users
//!    ↓
//! 3. OnlineManager Dispatches to Pushers
//!    For each registered MessagePusher:
//!    └─→ pusher.push(login_id, message)
//!    ↓
//! 4. Pusher Delivers Message
//!    ├─→ InMemoryPusher: Store in memory
//!    ├─→ WebSocketPusher: Send via WS
//!    └─→ Custom: Your implementation
//! ```
//!
//! ### Kick-Out Flow
//! ```text
//! 1. Trigger kick_out_notify(login_id, reason)
//!    ↓
//! 2. Create KickOut message
//!    └─→ message_type: MessageType::KickOut
//!    ↓
//! 3. Push notification to user
//!    └─→ All registered pushers receive message
//!    ↓
//! 4. Mark user offline
//!    └─→ mark_offline_all(login_id)
//! ```
//!
//! ### Message Types
//! - Text: Plain text messages
//! - Binary: Binary data
//! - KickOut: Force logout notification
//! - Notification: System notifications
//! - Custom: User-defined types
//!
//! ## 中文
//!
//! ### 概述
//! 本模块提供全面的在线用户管理和实时消息推送功能。
//! 它跟踪用户在线状态、管理连接，并实时向用户推送消息。
//!
//! ### 在线用户管理流程
//! ```text
//! 1. 用户连接（如 WebSocket、SSE）
//!    ↓
//! 2. OnlineManager.mark_online(OnlineUser)
//!    ├─→ 在 online_users HashMap 中存储用户信息
//!    ├─→ 键: login_id
//!    └─→ 值: Vec<OnlineUser>（支持多设备）
//!    ↓
//! 3. 用户活跃度更新
//!    OnlineManager.update_activity(login_id, token)
//!    └─→ 更新 last_activity 时间戳
//!    ↓
//! 4. 用户断开连接
//!    OnlineManager.mark_offline(login_id, token)
//!    ├─→ 从 online_users 中移除
//!    └─→ 如果没有更多会话则清理
//! ```
//!
//! ### 消息推送流程
//! ```text
//! 1. 创建 PushMessage
//!    ├─→ 生成唯一的 message_id
//!    ├─→ 设置 content 和 message_type
//!    └─→ 添加时间戳和元数据
//!    ↓
//! 2. 选择推送方法
//!    ├─→ push_to_user(login_id, content)      - 单个用户
//!    ├─→ push_to_users(vec![ids], content)    - 多个用户
//!    └─→ broadcast(content)                    - 所有在线用户
//!    ↓
//! 3. OnlineManager 分发给推送器
//!    对每个已注册的 MessagePusher:
//!    └─→ pusher.push(login_id, message)
//!    ↓
//! 4. 推送器传递消息
//!    ├─→ InMemoryPusher: 存储在内存
//!    ├─→ WebSocketPusher: 通过 WS 发送
//!    └─→ Custom: 你的实现
//! ```
//!
//! ### 强制下线流程
//! ```text
//! 1. 触发 kick_out_notify(login_id, reason)
//!    ↓
//! 2. 创建 KickOut 消息
//!    └─→ message_type: MessageType::KickOut
//!    ↓
//! 3. 推送通知给用户
//!    └─→ 所有已注册的推送器接收消息
//!    ↓
//! 4. 标记用户离线
//!    └─→ mark_offline_all(login_id)
//! ```
//!
//! ### 消息类型
//! - Text: 纯文本消息
//! - Binary: 二进制数据
//! - KickOut: 强制登出通知
//! - Notification: 系统通知
//! - Custom: 用户自定义类型

use crate::error::SaTokenError;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Online user information
/// 在线用户信息
///
/// Represents an active user connection with device and activity tracking
/// 表示具有设备和活动跟踪的活跃用户连接
#[derive(Debug, Clone)]
pub struct OnlineUser {
    /// User login ID | 用户登录 ID
    pub login_id: String,

    /// Authentication token | 认证 Token
    pub token: String,

    /// Device identifier (e.g., "web", "mobile", "ios", "android") | 设备标识
    pub device: String,

    /// Connection establishment time | 连接建立时间
    pub connect_time: DateTime<Utc>,

    /// Last activity timestamp | 最后活跃时间戳
    pub last_activity: DateTime<Utc>,

    /// Custom metadata for this connection | 该连接的自定义元数据
    pub metadata: HashMap<String, String>,
}

/// Push message structure
/// 推送消息结构
///
/// Represents a message to be delivered to online users
/// 表示要传递给在线用户的消息
#[derive(Debug, Clone)]
pub struct PushMessage {
    /// Unique message identifier | 唯一消息标识符
    pub message_id: String,

    /// Message content | 消息内容
    pub content: String,

    /// Message type | 消息类型
    pub message_type: MessageType,

    /// Message timestamp | 消息时间戳
    pub timestamp: DateTime<Utc>,

    /// Additional metadata | 额外元数据
    pub metadata: HashMap<String, String>,
}

/// Message type enumeration
/// 消息类型枚举
///
/// Defines different types of messages that can be sent
/// 定义可以发送的不同类型的消息
#[derive(Debug, Clone, PartialEq)]
pub enum MessageType {
    /// Plain text message | 纯文本消息
    Text,

    /// Binary data message | 二进制数据消息
    Binary,

    /// Force logout notification | 强制登出通知
    KickOut,

    /// System notification | 系统通知
    Notification,

    /// Custom message type | 自定义消息类型
    Custom(String),
}

/// Message pusher trait
/// 消息推送器 trait
///
/// Implement this trait to create custom message delivery mechanisms
/// 实现此 trait 以创建自定义消息传递机制
#[async_trait]
pub trait MessagePusher: Send + Sync {
    /// Push a message to a specific user
    /// 向特定用户推送消息
    ///
    /// # Arguments | 参数
    /// * `login_id` - User login ID | 用户登录 ID
    /// * `message` - Message to push | 要推送的消息
    async fn push(&self, login_id: &str, message: PushMessage) -> Result<(), SaTokenError>;
}

/// Online user manager
/// 在线用户管理器
///
/// Manages online users and handles real-time message pushing
/// 管理在线用户并处理实时消息推送
pub struct OnlineManager {
    /// Online users map: login_id -> Vec<OnlineUser>
    /// 在线用户映射: login_id -> Vec<OnlineUser>
    /// Supports multiple devices per user
    /// 支持每个用户多设备
    online_users: Arc<RwLock<HashMap<String, Vec<OnlineUser>>>>,

    /// Registered message pushers | 已注册的消息推送器
    pushers: Arc<RwLock<Vec<Arc<dyn MessagePusher>>>>,
}

impl OnlineManager {
    /// Create a new online manager
    /// 创建新的在线管理器
    ///
    /// # Example | 示例
    /// ```rust,ignore
    /// let manager = OnlineManager::new();
    /// ```
    pub fn new() -> Self {
        Self {
            online_users: Arc::new(RwLock::new(HashMap::new())),
            pushers: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Register a message pusher
    /// 注册消息推送器
    ///
    /// # Arguments | 参数
    /// * `pusher` - Message pusher implementation | 消息推送器实现
    ///
    /// # Example | 示例
    /// ```rust,ignore
    /// let pusher = Arc::new(MyCustomPusher::new());
    /// manager.register_pusher(pusher).await;
    /// ```
    pub async fn register_pusher(&self, pusher: Arc<dyn MessagePusher>) {
        let mut pushers = self.pushers.write().await;
        pushers.push(pusher);
    }

    /// Mark a user as online
    /// 标记用户上线
    ///
    /// # Arguments | 参数
    /// * `user` - Online user information | 在线用户信息
    ///
    /// # Example | 示例
    /// ```rust,ignore
    /// let user = OnlineUser {
    ///     login_id: "user123".to_string(),
    ///     token: "token123".to_string(),
    ///     device: "web".to_string(),
    ///     connect_time: Utc::now(),
    ///     last_activity: Utc::now(),
    ///     metadata: HashMap::new(),
    /// };
    /// manager.mark_online(user).await;
    /// ```
    pub async fn mark_online(&self, user: OnlineUser) {
        let mut users = self.online_users.write().await;
        users
            .entry(user.login_id.clone())
            .or_insert_with(Vec::new)
            .push(user);
    }

    /// Mark a specific user session as offline
    /// 标记特定用户会话离线
    ///
    /// # Arguments | 参数
    /// * `login_id` - User login ID | 用户登录 ID
    /// * `token` - Session token to remove | 要移除的会话 Token
    ///
    /// # Example | 示例
    /// ```rust,ignore
    /// manager.mark_offline("user123", "token123").await;
    /// ```
    pub async fn mark_offline(&self, login_id: &str, token: &str) {
        let mut users = self.online_users.write().await;
        if let Some(user_sessions) = users.get_mut(login_id) {
            user_sessions.retain(|u| u.token != token);
            if user_sessions.is_empty() {
                users.remove(login_id);
            }
        }
    }

    /// Mark all sessions of a user as offline
    /// 标记用户的所有会话离线
    ///
    /// # Arguments | 参数
    /// * `login_id` - User login ID | 用户登录 ID
    ///
    /// # Example | 示例
    /// ```rust,ignore
    /// manager.mark_offline_all("user123").await;
    /// ```
    pub async fn mark_offline_all(&self, login_id: &str) {
        let mut users = self.online_users.write().await;
        users.remove(login_id);
    }

    /// Check if a user is online
    /// 检查用户是否在线
    ///
    /// # Arguments | 参数
    /// * `login_id` - User login ID | 用户登录 ID
    ///
    /// # Returns | 返回值
    /// * `true` - User is online | 用户在线
    /// * `false` - User is offline | 用户离线
    ///
    /// # Example | 示例
    /// ```rust,ignore
    /// if manager.is_online("user123").await {
    ///     println!("User is online");
    /// }
    /// ```
    pub async fn is_online(&self, login_id: &str) -> bool {
        let users = self.online_users.read().await;
        users.contains_key(login_id)
    }

    /// Get online user count
    /// 获取在线用户数量
    ///
    /// # Returns | 返回值
    /// Number of online users | 在线用户数量
    ///
    /// # Example | 示例
    /// ```rust,ignore
    /// let count = manager.get_online_count().await;
    /// println!("{} users online", count);
    /// ```
    pub async fn get_online_count(&self) -> usize {
        let users = self.online_users.read().await;
        users.len()
    }

    /// Get list of online user IDs
    /// 获取在线用户 ID 列表
    ///
    /// # Returns | 返回值
    /// Vector of login IDs | 登录 ID 向量
    ///
    /// # Example | 示例
    /// ```rust,ignore
    /// let users = manager.get_online_users().await;
    /// for user_id in users {
    ///     println!("User {} is online", user_id);
    /// }
    /// ```
    pub async fn get_online_users(&self) -> Vec<String> {
        let users = self.online_users.read().await;
        users.keys().cloned().collect()
    }

    /// Get all sessions for a specific user
    /// 获取特定用户的所有会话
    ///
    /// # Arguments | 参数
    /// * `login_id` - User login ID | 用户登录 ID
    ///
    /// # Returns | 返回值
    /// Vector of online user sessions | 在线用户会话向量
    ///
    /// # Example | 示例
    /// ```rust,ignore
    /// let sessions = manager.get_user_sessions("user123").await;
    /// println!("User has {} active sessions", sessions.len());
    /// ```
    pub async fn get_user_sessions(&self, login_id: &str) -> Vec<OnlineUser> {
        let users = self.online_users.read().await;
        users.get(login_id).cloned().unwrap_or_default()
    }

    /// Update user activity timestamp
    /// 更新用户活跃时间戳
    ///
    /// # Arguments | 参数
    /// * `login_id` - User login ID | 用户登录 ID
    /// * `token` - Session token | 会话 Token
    ///
    /// # Example | 示例
    /// ```rust,ignore
    /// manager.update_activity("user123", "token123").await;
    /// ```
    pub async fn update_activity(&self, login_id: &str, token: &str) {
        let mut users = self.online_users.write().await;
        if let Some(user_sessions) = users.get_mut(login_id) {
            for user in user_sessions.iter_mut() {
                if user.token == token {
                    user.last_activity = Utc::now();
                    break;
                }
            }
        }
    }

    /// Push a text message to a specific user
    /// 向特定用户推送文本消息
    ///
    /// # Arguments | 参数
    /// * `login_id` - User login ID | 用户登录 ID
    /// * `content` - Message content | 消息内容
    ///
    /// # Example | 示例
    /// ```rust,ignore
    /// manager.push_to_user("user123", "Hello!".to_string()).await?;
    /// ```
    pub async fn push_to_user(&self, login_id: &str, content: String) -> Result<(), SaTokenError> {
        let message = PushMessage {
            message_id: uuid::Uuid::new_v4().to_string(),
            content,
            message_type: MessageType::Text,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };

        let pushers = self.pushers.read().await;
        for pusher in pushers.iter() {
            pusher.push(login_id, message.clone()).await?;
        }

        Ok(())
    }

    /// Push a message to multiple users
    /// 向多个用户推送消息
    ///
    /// # Arguments | 参数
    /// * `login_ids` - List of user login IDs | 用户登录 ID 列表
    /// * `content` - Message content | 消息内容
    ///
    /// # Example | 示例
    /// ```rust,ignore
    /// let users = vec!["user1".to_string(), "user2".to_string()];
    /// manager.push_to_users(users, "Broadcast!".to_string()).await?;
    /// ```
    pub async fn push_to_users(
        &self,
        login_ids: Vec<String>,
        content: String,
    ) -> Result<(), SaTokenError> {
        for login_id in login_ids {
            self.push_to_user(&login_id, content.clone()).await?;
        }
        Ok(())
    }

    /// Broadcast a message to all online users
    /// 向所有在线用户广播消息
    ///
    /// # Arguments | 参数
    /// * `content` - Message content | 消息内容
    ///
    /// # Example | 示例
    /// ```rust,ignore
    /// manager.broadcast("System maintenance in 5 minutes".to_string()).await?;
    /// ```
    pub async fn broadcast(&self, content: String) -> Result<(), SaTokenError> {
        let login_ids = self.get_online_users().await;
        self.push_to_users(login_ids, content).await
    }

    /// Push a custom message to a user
    /// 向用户推送自定义消息
    ///
    /// # Arguments | 参数
    /// * `login_id` - User login ID | 用户登录 ID
    /// * `message` - Custom push message | 自定义推送消息
    ///
    /// # Example | 示例
    /// ```rust,ignore
    /// let message = PushMessage {
    ///     message_id: uuid::Uuid::new_v4().to_string(),
    ///     content: "Custom content".to_string(),
    ///     message_type: MessageType::Custom("event".to_string()),
    ///     timestamp: Utc::now(),
    ///     metadata: HashMap::new(),
    /// };
    /// manager.push_message_to_user("user123", message).await?;
    /// ```
    pub async fn push_message_to_user(
        &self,
        login_id: &str,
        message: PushMessage,
    ) -> Result<(), SaTokenError> {
        let pushers = self.pushers.read().await;
        for pusher in pushers.iter() {
            pusher.push(login_id, message.clone()).await?;
        }
        Ok(())
    }

    /// Kick out a user and send notification
    /// 踢出用户并发送通知
    ///
    /// # Arguments | 参数
    /// * `login_id` - User login ID | 用户登录 ID
    /// * `reason` - Kick-out reason | 踢出原因
    ///
    /// # Example | 示例
    /// ```rust,ignore
    /// manager.kick_out_notify("user123", "Duplicate login detected".to_string()).await?;
    /// ```
    pub async fn kick_out_notify(
        &self,
        login_id: &str,
        reason: String,
    ) -> Result<(), SaTokenError> {
        // Create kick-out message | 创建踢出消息
        let message = PushMessage {
            message_id: uuid::Uuid::new_v4().to_string(),
            content: reason,
            message_type: MessageType::KickOut,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };

        // Push notification | 推送通知
        self.push_message_to_user(login_id, message).await?;

        // Mark user offline | 标记用户离线
        self.mark_offline_all(login_id).await;
        Ok(())
    }
}

impl Default for OnlineManager {
    fn default() -> Self {
        Self::new()
    }
}

/// In-memory message pusher implementation
/// 内存消息推送器实现
///
/// Stores messages in memory for testing and development
/// 在内存中存储消息用于测试和开发
pub struct InMemoryPusher {
    /// Messages storage: login_id -> Vec<PushMessage>
    /// 消息存储: login_id -> Vec<PushMessage>
    messages: Arc<RwLock<HashMap<String, Vec<PushMessage>>>>,
}

impl InMemoryPusher {
    /// Create a new in-memory pusher
    /// 创建新的内存推送器
    pub fn new() -> Self {
        Self {
            messages: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get all messages for a user
    /// 获取用户的所有消息
    ///
    /// # Arguments | 参数
    /// * `login_id` - User login ID | 用户登录 ID
    ///
    /// # Returns | 返回值
    /// Vector of push messages | 推送消息向量
    pub async fn get_messages(&self, login_id: &str) -> Vec<PushMessage> {
        let messages = self.messages.read().await;
        messages.get(login_id).cloned().unwrap_or_default()
    }

    /// Clear all messages for a user
    /// 清除用户的所有消息
    ///
    /// # Arguments | 参数
    /// * `login_id` - User login ID | 用户登录 ID
    pub async fn clear_messages(&self, login_id: &str) {
        let mut messages = self.messages.write().await;
        messages.remove(login_id);
    }
}

impl Default for InMemoryPusher {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MessagePusher for InMemoryPusher {
    async fn push(&self, login_id: &str, message: PushMessage) -> Result<(), SaTokenError> {
        let mut messages = self.messages.write().await;
        messages
            .entry(login_id.to_string())
            .or_insert_with(Vec::new)
            .push(message);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_online_manager() {
        let manager = OnlineManager::new();

        let user = OnlineUser {
            login_id: "user1".to_string(),
            token: "token1".to_string(),
            device: "web".to_string(),
            connect_time: Utc::now(),
            last_activity: Utc::now(),
            metadata: HashMap::new(),
        };

        manager.mark_online(user).await;

        assert!(manager.is_online("user1").await);
        assert_eq!(manager.get_online_count().await, 1);
    }

    #[tokio::test]
    async fn test_mark_offline() {
        let manager = OnlineManager::new();

        let user = OnlineUser {
            login_id: "user2".to_string(),
            token: "token2".to_string(),
            device: "mobile".to_string(),
            connect_time: Utc::now(),
            last_activity: Utc::now(),
            metadata: HashMap::new(),
        };

        manager.mark_online(user).await;
        assert!(manager.is_online("user2").await);

        manager.mark_offline("user2", "token2").await;
        assert!(!manager.is_online("user2").await);
    }

    #[tokio::test]
    async fn test_push_message() {
        let manager = OnlineManager::new();
        let pusher = Arc::new(InMemoryPusher::new());

        manager.register_pusher(pusher.clone()).await;

        let user = OnlineUser {
            login_id: "user3".to_string(),
            token: "token3".to_string(),
            device: "web".to_string(),
            connect_time: Utc::now(),
            last_activity: Utc::now(),
            metadata: HashMap::new(),
        };

        manager.mark_online(user).await;
        manager
            .push_to_user("user3", "Hello".to_string())
            .await
            .unwrap();

        let messages = pusher.get_messages("user3").await;
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "Hello");
    }

    #[tokio::test]
    async fn test_broadcast() {
        let manager = OnlineManager::new();
        let pusher = Arc::new(InMemoryPusher::new());

        manager.register_pusher(pusher.clone()).await;

        for i in 1..=3 {
            let user = OnlineUser {
                login_id: format!("user{}", i),
                token: format!("token{}", i),
                device: "web".to_string(),
                connect_time: Utc::now(),
                last_activity: Utc::now(),
                metadata: HashMap::new(),
            };
            manager.mark_online(user).await;
        }

        manager
            .broadcast("Broadcast message".to_string())
            .await
            .unwrap();

        for i in 1..=3 {
            let messages = pusher.get_messages(&format!("user{}", i)).await;
            assert_eq!(messages.len(), 1);
        }
    }

    #[tokio::test]
    async fn test_kick_out_notify() {
        let manager = OnlineManager::new();
        let pusher = Arc::new(InMemoryPusher::new());

        manager.register_pusher(pusher.clone()).await;

        let user = OnlineUser {
            login_id: "user4".to_string(),
            token: "token4".to_string(),
            device: "web".to_string(),
            connect_time: Utc::now(),
            last_activity: Utc::now(),
            metadata: HashMap::new(),
        };

        manager.mark_online(user).await;
        assert!(manager.is_online("user4").await);

        manager
            .kick_out_notify("user4", "Kicked out".to_string())
            .await
            .unwrap();

        assert!(!manager.is_online("user4").await);

        let messages = pusher.get_messages("user4").await;
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].message_type, MessageType::KickOut);
    }
}
