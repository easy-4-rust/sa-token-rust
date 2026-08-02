# sa-token-rust

一个轻量级、高性能的 Rust 认证授权框架，灵感来自 [sa-token](https://github.com/dromara/sa-token)。

<div style="margin: 24px 0;">
  <a href="../" style="display: inline-block; padding: 8px 16px; border: 1px solid #ccc; border-radius: 6px; text-decoration: none; margin-right: 8px;">📖 English</a>
  <a href="./guide/quick-start" style="display: inline-block; padding: 8px 16px; border: 1px solid #ccc; border-radius: 6px; text-decoration: none;">🚀 快速开始</a>
</div>

## 特性

- 🚀 **9 种 Web 框架支持**：Axum, Actix-web, Poem, Rocket, Warp, Salvo, Tide, Gotham, Ntex
- 🔐 **完整的认证**：登录、登出、Token 验证、Session 管理
- 🛡️ **细粒度授权**：基于权限和角色的访问控制，支持通配符匹配
- 💾 **灵活存储**：内存、Redis 和数据库存储后端
- 🎯 **易于使用**：过程宏和工具类简化集成
- ⚡ **高性能**：零拷贝设计，支持 async/await
- 🔧 **高度可配置**：Token 超时、Cookie 选项、自定义 Token 名称
- 🎧 **事件监听**：监听登录、登出、踢出下线等认证事件
- 🔑 **JWT 支持**：完整的 JWT 实现，支持 8 种算法（HS256/384/512、RS256/384/512、ES256/384）
- 🔒 **安全特性**：Nonce 防重放攻击、Refresh Token 刷新机制
- 🌐 **OAuth2 支持**：完整的 OAuth2 授权码模式
- 🌐 **WebSocket 认证**：安全的 WebSocket 连接认证，支持多种 Token 来源
- 👥 **在线用户管理**：实时在线状态跟踪和消息推送
- 🔄 **分布式 Session**：跨服务 Session 共享，适用于微服务架构
- 🎫 **SSO 单点登录**：基于票据的 SSO 和统一登出

---


## 项目结构

```
sa-token-rust/
├── sa-token-core/                     # 核心库（Token、Session、Manager、Router）
├── sa-token-adapter/                  # 适配器接口（Storage、Request/Response）
├── sa-token-macro/                    # 过程宏（#[sa_check_login] 等）
├── sa-token-storage/                  # 存储后端
│   ├── sa-token-storage-memory/         # 内存存储
│   ├── sa-token-storage-redis/          # Redis 存储（+ 构建器）
│   └── sa-token-storage-database/       # 数据库存储（占位）
├── sa-token-plugin/                   # Web 框架集成
│   ├── sa-token-plugin-actix-web/     # Actix-web 门面（默认 v4）
│   ├── sa-token-plugin-axum/          # Axum 集成（v8）
│   ├── sa-token-plugin-gotham/        # Gotham 门面（默认 v074）
│   ├── sa-token-plugin-ntex/          # Ntex 门面（默认 v212）
│   ├── sa-token-plugin-poem/          # Poem 集成
│   ├── sa-token-plugin-rocket/        # Rocket 门面（默认 v05）
│   ├── sa-token-plugin-salvo/         # Salvo 门面（默认 v079）
│   ├── sa-token-plugin-tide/          # Tide 集成
│   └── sa-token-plugin-warp/          # Warp 集成
├── sa-token-examples/                          # 示例项目
```

> **版本分离**：门面 crate 通过 Cargo features 在编译时选择框架大版本（`v4`/`v5`、`v05`、`v079` 等）。

## 解决的问题

### 1. 框架集成复杂性
9 个 Web 框架，一套统一 API。每个插件提供相同的中间件 + 提取器 + Token 提取模式。

### 2. 减少样板代码
声明式宏消除手动认证检查：
```rust
#[sa_check_permission("user:delete")]
async fn delete_user() { /* 自动检查 */ }
```

### 3. Session 和 Token 管理
```rust
let token = StpUtil::login("user_10001").await?;
StpUtil::set_roles("user_10001", vec!["admin".into()]).await?;
StpUtil::logout(&token).await?;
```

### 4. 权限和角色系统
内置通配符匹配（`user:*` 匹配 `user:list`、`user:delete`），支持 AND/OR 逻辑。

### 5. 分布式和 SSO
跨服务 Session 共享 + 基于票据的单点登录，适配微服务架构。

### 6. WebSocket 认证
专用 `WsAuthManager`，从 header/query/cookie 认证 WebSocket 连接。

### 7. 安全特性
Nonce 防重放、Refresh Token 刷新、JWT 签名（8 种算法）。

### 8. 事件系统
```rust
impl SaTokenListener for MyListener {
    async fn on_login(&self, login_id: &str, token: &str, login_type: &str) {
        // 记录日志、发送通知、审计
    }
}
StpUtil::register_listener(Arc::new(MyListener));
```

## 🚀 快速入门

只需添加一个依赖即可开始：

```toml
[dependencies]
sa-token-plugin-axum = "0.1.14"
tokio = { version = "1", features = ["full"] }
axum = "0.8"
```

```rust
use sa_token_plugin_axum::*;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let state = SaTokenState::builder()
        .storage(Arc::new(MemoryStorage::new()))
        .token_name("Authorization")
        .timeout(86400)
        .build();

    let app = axum::Router::new()
        .route("/user/info", axum::routing::get(user_info))
        .layer(SaTokenMiddleware::new(state));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn user_info(LoginIdExtractor(login_id): LoginIdExtractor) -> String {
    StpUtil::login(&login_id).await.unwrap();
    format!("当前用户: {}", login_id)
}
```

**通过 features 选择存储后端：**

```toml
# Redis 存储
sa-token-plugin-axum = { version = "0.1.14", features = ["redis"] }

# 所有后端
sa-token-plugin-axum = { version = "0.1.14", features = ["full"] }
```

**可用的插件：** `sa-token-plugin-axum`、`sa-token-plugin-actix-web`（默认 v4）、`sa-token-plugin-poem`、`sa-token-plugin-rocket`（默认 v05）、`sa-token-plugin-warp`、`sa-token-plugin-salvo`（默认 v079）、`sa-token-plugin-tide`、`sa-token-plugin-gotham`（默认 v074）、`sa-token-plugin-ntex`（默认 v212）

➡️ **[完整快速入门指南 →](/zh/guide/quick-start.md)**

---

## 📖 基础

日常使用的核心概念和 API。

### StpUtil — 主 API

`StpUtil` 是所有认证和授权操作的主要工具类：

```rust
use sa_token_core::StpUtil;

// 登录
let token = StpUtil::login("user_10001").await?;

// 设置权限和角色
StpUtil::set_permissions("user_10001", vec!["user:list".into(), "user:add".into()]).await?;
StpUtil::set_roles("user_10001", vec!["admin".into()]).await?;

// 检查登录状态
StpUtil::is_login_by_login_id("user_10001").await;

// 检查权限
StpUtil::has_permission("user_10001", "user:list").await;
StpUtil::has_role("user_10001", "admin").await;

// 登出
StpUtil::logout(&token).await?;
```

➡️ **[StpUtil API 参考 →](/zh/guide/stp-util.md)**

### 过程宏

8 个声明式认证宏 — `#[sa_check_login]`、`#[sa_check_permission]`、`#[sa_check_role]`、AND/OR 变体、`#[sa_ignore]`：

```rust
use sa_token_macro::*;

#[sa_check_login]
async fn protected_route() -> &'static str { "此路由需要登录" }

#[sa_check_permission("user:delete")]
async fn delete_user(user_id: String) -> &'static str { "用户已删除" }

#[sa_check_role("admin")]
async fn admin_only() -> &'static str { "仅管理员可见" }
```

➡️ **[过程宏 →](/zh/guide/permission-matching)**

### 过程宏

```rust
use sa_token_macro::*;

#[sa_check_login]
async fn protected_route() -> &'static str { "此路由需要登录" }

#[sa_check_permission("user:delete")]
async fn delete_user(user_id: String) -> &'static str { "用户已删除" }

#[sa_check_role("admin")]
async fn admin_only() -> &'static str { "仅管理员可见" }

#[sa_check_permissions_or("user:read", "user:write")]
async fn user_access() -> &'static str { "具有读或写权限" }
```

### 事件监听

实时监听认证事件：

```rust
use sa_token_core::{SaTokenListener, LoggingListener};

struct MyListener;

#[async_trait]
impl SaTokenListener for MyListener {
    async fn on_login(&self, login_id: &str, token: &str, login_type: &str) {
        println!("用户 {} 登录了", login_id);
    }
    async fn on_logout(&self, login_id: &str, token: &str, login_type: &str) {
        println!("用户 {} 登出了", login_id);
    }
    async fn on_kick_out(&self, login_id: &str, token: &str, login_type: &str) {
        println!("用户 {} 被踢出下线", login_id);
    }
}

StpUtil::register_listener(Arc::new(MyListener)).await;
```

➡️ **[事件监听指南 →](/zh/guide/event-listener)**

### 路径鉴权

基于 URL 路径模式配置认证规则：

➡️ **[路径鉴权指南 →](/zh/guide/path-auth.md)**

### Token 风格

从 9 种 Token 生成风格中选择：Uuid、SimpleUuid、Random32/64/128、Jwt、Hash、Timestamp、Tik。

```rust
let config = SaTokenConfig::builder()
    .token_style(TokenStyle::Tik)  // 短小8字符 Token
    .build_config();
```

➡️ **[Token 风格参考 →](/zh/guide/token-styles.md)**

---

## 🎯 进阶

### JWT（JSON Web Token）

完整的 JWT 支持，包含 8 种算法（HS256/384/512、RS256/384/512、ES256/384）和自定义声明：

```rust
let config = SaTokenConfig::builder()
    .token_style(TokenStyle::Jwt)
    .jwt_secret_key("your-secret-key")
    .build_config();
```

➡️ **[JWT 指南 →](/zh/guide/jwt.md)**

### OAuth2

完整的 OAuth2 授权码模式：

```rust
let oauth2 = OAuth2Manager::new(storage);
let client = OAuth2Client { client_id, client_secret, redirect_uris, grant_types, scope };
oauth2.register_client(&client).await?;
let token = oauth2.exchange_code_for_token(&code, &client_id, &secret, &redirect).await?;
```

➡️ **[OAuth2 指南 →](/zh/guide/oauth2.md)**

### 安全特性

Nonce 防重放攻击和 Refresh Token 刷新机制：

```rust
let nonce_manager = NonceManager::new(storage, 300);
nonce_manager.validate_and_consume(&nonce, "user_123").await?;

let refresh_manager = RefreshTokenManager::new(storage, config);
let (new_token, user_id) = refresh_manager.refresh_access_token(&refresh_token).await?;
```

➡️ **[安全特性 →](/zh/guide/security-features.md)**

### WebSocket 认证

使用多种 Token 来源（header、query、cookie）认证 WebSocket 连接：

➡️ **[WebSocket 认证 →](/zh/guide/websocket-auth.md)**

### 在线用户管理

实时用户追踪和消息推送：

➡️ **[在线用户管理 →](/zh/guide/online-user-management.md)**

### 分布式 Session

微服务架构的跨服务 Session 共享：

➡️ **[分布式 Session →](/zh/guide/distributed-session.md)**

### SSO 单点登录

完整的 SSO 实现，支持票据认证和统一登出：

```rust
let sso_server = SsoServer::new(manager.clone()).with_ticket_timeout(300);
let client = SsoClient::new(manager.clone(), "http://sso.example.com/auth", "http://app1.example.com");
let ticket = sso_server.login("user_123", "http://app1.example.com").await?;
let login_id = sso_server.validate_ticket(&ticket.ticket_id, "http://app1.example.com").await?;
```

➡️ **[SSO 指南 →](/zh/guide/sso.md#中文)**

### 自定义存储

实现 `SaStorage` trait 使用自己的存储后端：

```rust
use sa_token_adapter::storage::SaStorage;

#[async_trait]
impl SaStorage for CustomStorage {
    async fn get(&self, key: &str) -> Option<String> { /* ... */ }
    async fn set(&self, key: &str, value: String, timeout: Option<i64>) { /* ... */ }
    // ... 其他方法
}
```

### 版本分离插件架构

门面 crate 使用 Cargo features 在编译时选择框架版本：

| 门面 Crate | Feature（默认） | 绑定 Crate |
|---|---|---|
| `sa-token-plugin-actix-web` | `v4` | `sa-token-plugin-actix-web-v4` |
| `sa-token-plugin-rocket` | `v05` | `sa-token-plugin-rocket-v05` |
| `sa-token-plugin-salvo` | `v079` | `sa-token-plugin-salvo-v079` |
| `sa-token-plugin-gotham` | `v074` | `sa-token-plugin-gotham-v074` |
| `sa-token-plugin-ntex` | `v212` | `sa-token-plugin-ntex-v212` |

每个门面还共享一个 `-core` crate 用于通用逻辑（状态、适配器、错误响应）。

### 框架集成

所有 9 种支持 Web 框架的完整示例：

➡️ **[框架集成 →](/zh/guide/framework-integration.md)**

### 错误参考

涵盖所有错误类型的完整错误码参考：

➡️ **[错误参考 →](/zh/reference/error-reference.md#中文)**

### 项目介绍

---

## 支持的语言

所有文档支持以下语言：

- 🇬🇧 English（英语）
- 🇨🇳 中文

---

## 许可证

本项目采用双重许可：

- **Apache License, Version 2.0** ([LICENSE-APACHE](https://github.com/sa-tokens/sa-token-rust/blob/main/LICENSE-APACHE))
- **MIT License** ([LICENSE-MIT](https://github.com/sa-tokens/sa-token-rust/blob/main/LICENSE-MIT))

你可以任选其一。

---

## 社区交流

扫码加入微信群：

![sa-token-rust 微信交流群](../IMG_3972.JPG)

---

**由 sa-tokens 社区用 ❤️ 制作**
