# OAuth2 授权码模式完整指南

中文 | [English](./OAUTH2_GUIDE.md)

---

## 概述

sa-token-rust 实现了完整的 OAuth2 授权码模式（Authorization Code Grant），符合 RFC 6749 标准，支持第三方应用授权、单点登录、API 访问控制等场景。

## 目录

- [功能特性](#功能特性)
- [快速开始](#快速开始)
- [核心组件](#核心组件)
- [授权流程](#授权流程)
- [API 参考](#api-参考)
- [安全最佳实践](#安全最佳实践)

## 功能特性

- ✅ 符合 OAuth2 RFC 6749 标准
- ✅ 授权码模式（Authorization Code Grant）
- ✅ 客户端管理（注册、验证）
- ✅ 授权码生成与验证
- ✅ 访问令牌管理
- ✅ 刷新令牌机制
- ✅ Redirect URI 严格验证
- ✅ Scope 权限控制
- ✅ 令牌撤销
- ✅ 自动过期清理

## 快速开始

### 1. 创建 OAuth2 管理器

```rust
use sa_token_core::OAuth2Manager;
use std::sync::Arc;

let storage = Arc::new(MemoryStorage::new());
let oauth2 = OAuth2Manager::new(storage)
    .with_ttl(
        600,      // 授权码 10 分钟
        3600,     // 访问令牌 1 小时
        2592000   // 刷新令牌 30 天
    );
```

### 2. 注册客户端

```rust
use sa_token_core::OAuth2Client;

let client = OAuth2Client {
    client_id: "web_app_001".to_string(),
    client_secret: "secret_abc123xyz".to_string(),
    redirect_uris: vec![
        "http://localhost:3000/callback".to_string(),
    ],
    grant_types: vec![
        "authorization_code".to_string(),
        "refresh_token".to_string(),
    ],
    scope: vec![
        "read".to_string(),
        "write".to_string(),
        "profile".to_string(),
    ],
};

oauth2.register_client(&client).await?;
```

### 3. 完整授权流程

```rust
// 步骤 1: 生成授权码（用户同意授权后）
let auth_code = oauth2.generate_authorization_code(
    "web_app_001".to_string(),
    "user_123".to_string(),
    "http://localhost:3000/callback".to_string(),
    vec!["read".to_string(), "profile".to_string()],
);

oauth2.store_authorization_code(&auth_code).await?;

// 步骤 2: 授权码换取访问令牌
let token = oauth2.exchange_code_for_token(
    &auth_code.code,
    "web_app_001",
    "secret_abc123xyz",
    "http://localhost:3000/callback",
).await?;

// 步骤 3: 使用访问令牌
let token_info = oauth2.verify_access_token(&token.access_token).await?;

// 步骤 4: 刷新令牌
let new_token = oauth2.refresh_access_token(
    token.refresh_token.as_ref().unwrap(),
    "web_app_001",
    "secret_abc123xyz",
).await?;
```

## 核心组件

### OAuth2Manager

OAuth2 管理器，负责整个授权流程的管理。

```rust
pub struct OAuth2Manager {
    storage: Arc<dyn SaStorage>,
    code_ttl: i64,
    token_ttl: i64,
    refresh_token_ttl: i64,
}
```

**方法列表**：
- `new(storage)` - 创建管理器
- `with_ttl(code_ttl, token_ttl, refresh_ttl)` - 设置过期时间
- `register_client(&client)` - 注册客户端
- `get_client(client_id)` - 获取客户端信息
- `verify_client(client_id, client_secret)` - 验证客户端凭据
- `generate_authorization_code(...)` - 生成授权码
- `store_authorization_code(&code)` - 存储授权码
- `exchange_code_for_token(...)` - 授权码换令牌
- `verify_access_token(&token)` - 验证访问令牌
- `refresh_access_token(...)` - 刷新访问令牌
- `revoke_token(&token)` - 撤销令牌

### OAuth2Client

客户端信息。

```rust
pub struct OAuth2Client {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uris: Vec<String>,
    pub grant_types: Vec<String>,
    pub scope: Vec<String>,
}
```

### AuthorizationCode

授权码信息。

```rust
pub struct AuthorizationCode {
    pub code: String,
    pub client_id: String,
    pub user_id: String,
    pub redirect_uri: String,
    pub scope: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}
```

### AccessToken

访问令牌响应。

```rust
pub struct AccessToken {
    pub access_token: String,
    pub token_type: String,        // "Bearer"
    pub expires_in: i64,
    pub refresh_token: Option<String>,
    pub scope: Vec<String>,
}
```

## 授权流程

### 完整流程图

```
┌─────────┐                               ┌─────────────┐
│  用户   │                               │  客户端应用  │
└────┬────┘                               └──────┬──────┘
     │                                            │
     │  1. 访问第三方应用                         │
     │───────────────────────────────────────────▶│
     │                                            │
     │  2. 重定向到授权页面                       │
     │◀───────────────────────────────────────────│
     │                                            │
┌────▼────┐                               ┌──────┴──────┐
│ 授权服务器│                               │   资源服务器 │
└────┬────┘                               └──────┬──────┘
     │                                            │
     │  3. 用户同意授权                           │
     │                                            │
     │  4. 生成授权码                             │
     │  oauth2.generate_authorization_code()      │
     │                                            │
     │  5. 重定向回客户端（带授权码）             │
     │───────────────────────────────────────────▶│
     │                                            │
     │  6. 使用授权码换取访问令牌                 │
     │  oauth2.exchange_code_for_token()          │
     │◀───────────────────────────────────────────│
     │                                            │
     │  7. 返回访问令牌和刷新令牌                 │
     │───────────────────────────────────────────▶│
     │                                            │
     │  8. 使用访问令牌访问资源                   │
     │                                            │───────▶
     │                                            │
     │  9. 返回资源                               │
     │                                            │◀───────
     │                                            │
     │  10. 访问令牌过期，使用刷新令牌获取新令牌   │
     │  oauth2.refresh_access_token()             │
     │◀───────────────────────────────────────────│
     │                                            │
     │  11. 返回新的访问令牌                      │
     │───────────────────────────────────────────▶│
     │                                            │
```

### 详细步骤说明

#### 步骤 1: 用户访问第三方应用

用户点击"使用 XXX 登录"按钮。

#### 步骤 2: 重定向到授权页面

```
https://auth-server.com/oauth2/authorize?
    client_id=web_app_001&
    redirect_uri=http://localhost:3000/callback&
    response_type=code&
    scope=read+profile&
    state=random_state
```

#### 步骤 3: 用户同意授权

用户在授权页面确认授权范围并同意。

#### 步骤 4: 生成授权码

```rust
let auth_code = oauth2.generate_authorization_code(
    client_id,
    user_id,
    redirect_uri,
    scope,
);
oauth2.store_authorization_code(&auth_code).await?;
```

#### 步骤 5: 重定向回客户端

```
http://localhost:3000/callback?
    code=code_1dc590a362b04f919a64aab5d54218de&
    state=random_state
```

#### 步骤 6-7: 授权码换取令牌

```rust
let token = oauth2.exchange_code_for_token(
    &code,
    &client_id,
    &client_secret,
    &redirect_uri,
).await?;

// 响应：
// {
//   "access_token": "at_3bf203bd7bba452b9acf189445912f25",
//   "token_type": "Bearer",
//   "expires_in": 3600,
//   "refresh_token": "rt_03d7f54eeca2492f989e44dad369a223",
//   "scope": ["read", "profile"]
// }
```

#### 步骤 8-9: 访问资源

```rust
let token_info = oauth2.verify_access_token(&access_token).await?;
// 验证成功，可以访问用户资源
```

#### 步骤 10-11: 刷新令牌

```rust
let new_token = oauth2.refresh_access_token(
    &refresh_token,
    &client_id,
    &client_secret,
).await?;
```

## API 参考

### 客户端管理

#### register_client

注册 OAuth2 客户端。

```rust
pub async fn register_client(&self, client: &OAuth2Client) -> SaTokenResult<()>
```

#### get_client

获取客户端信息。

```rust
pub async fn get_client(&self, client_id: &str) -> SaTokenResult<OAuth2Client>
```

#### verify_client

验证客户端凭据。

```rust
pub async fn verify_client(&self, client_id: &str, client_secret: &str) -> SaTokenResult<bool>
```

### 授权码管理

#### generate_authorization_code

生成授权码。

```rust
pub fn generate_authorization_code(
    &self,
    client_id: String,
    user_id: String,
    redirect_uri: String,
    scope: Vec<String>,
) -> AuthorizationCode
```

#### store_authorization_code

存储授权码。

```rust
pub async fn store_authorization_code(&self, auth_code: &AuthorizationCode) -> SaTokenResult<()>
```

#### exchange_code_for_token

使用授权码换取访问令牌。

```rust
pub async fn exchange_code_for_token(
    &self,
    code: &str,
    client_id: &str,
    client_secret: &str,
    redirect_uri: &str,
) -> SaTokenResult<AccessToken>
```

### 令牌管理

#### generate_access_token

生成访问令牌。

```rust
pub async fn generate_access_token(
    &self,
    client_id: &str,
    user_id: &str,
    scope: Vec<String>,
) -> SaTokenResult<AccessToken>
```

#### verify_access_token

验证访问令牌。

```rust
pub async fn verify_access_token(&self, access_token: &str) -> SaTokenResult<OAuth2TokenInfo>
```

#### refresh_access_token

刷新访问令牌。

```rust
pub async fn refresh_access_token(
    &self,
    refresh_token: &str,
    client_id: &str,
    client_secret: &str,
) -> SaTokenResult<AccessToken>
```

#### revoke_token

撤销令牌。

```rust
pub async fn revoke_token(&self, token: &str) -> SaTokenResult<()>
```

### 验证方法

#### validate_redirect_uri

验证回调 URI 是否在允许列表中。

```rust
pub fn validate_redirect_uri(&self, client: &OAuth2Client, redirect_uri: &str) -> bool
```

#### validate_scope

验证请求的权限范围是否合法。

```rust
pub fn validate_scope(&self, client: &OAuth2Client, requested_scope: &[String]) -> bool
```

## 安全最佳实践

### 1. 客户端凭据

- ✅ 使用强密钥作为 client_secret（至少 32 个字符）
- ✅ 定期轮换客户端密钥
- ✅ 安全存储客户端凭据，不要硬编码
- ✅ 使用 HTTPS 传输客户端凭据

### 2. 授权码

- ✅ 授权码只能使用一次（已实现）
- ✅ 授权码有效期短（默认 10 分钟）
- ✅ 验证 redirect_uri 严格匹配（已实现）
- ✅ 使用 state 参数防止 CSRF 攻击

### 3. 访问令牌

- ✅ 访问令牌有效期短（推荐 1-2 小时）
- ✅ 使用 Bearer Token 格式
- ✅ 验证令牌签名和过期时间
- ✅ 实现令牌撤销机制（已实现）

### 4. 刷新令牌

- ✅ 刷新令牌有效期长（7-30 天）
- ✅ 安全存储刷新令牌
- ✅ 刷新时验证客户端凭据（已实现）
- ✅ 刷新后可选择性撤销旧刷新令牌

### 5. Redirect URI

- ✅ 严格验证 redirect_uri（已实现）
- ✅ 使用白名单机制（已实现）
- ✅ 禁止使用通配符
- ✅ 推荐使用 HTTPS

### 6. Scope

- ✅ 实现最小权限原则
- ✅ 验证请求的 scope 是否合法（已实现）
- ✅ 明确告知用户授权范围
- ✅ 支持 scope 降级

### 7. 传输安全

- ✅ **必须使用 HTTPS**
- ✅ 启用 HSTS
- ✅ 使用安全的 TLS 版本（1.2+）
- ✅ 验证 SSL 证书

## 错误处理

```rust
use sa_token_core::SaTokenError;

match oauth2.exchange_code_for_token(&code, &client_id, &client_secret, &redirect_uri).await {
    Ok(token) => {
        // 成功获取令牌
        println!("Access Token: {}", token.access_token);
    }
    Err(SaTokenError::InvalidToken(msg)) => {
        // 无效的授权码或客户端凭据
        eprintln!("Invalid: {}", msg);
    }
    Err(SaTokenError::TokenExpired) => {
        // 授权码已过期
        eprintln!("Authorization code expired");
    }
    Err(e) => {
        // 其他错误
        eprintln!("Error: {:?}", e);
    }
}
```

## 示例

运行完整示例：

```bash
cargo run --example oauth2_example
```

查看示例代码：`sa-token-examples/oauth2_example.rs`

## 常见问题

### 1. 授权码只能使用一次吗？

是的。`exchange_code_for_token` 方法会调用 `consume_authorization_code`，授权码在使用后立即删除。

### 2. 刷新令牌会过期吗？

会的。刷新令牌默认有效期为 30 天，过期后用户需要重新授权。

### 3. 如何自定义令牌有效期？

```rust
let oauth2 = OAuth2Manager::new(storage)
    .with_ttl(
        300,      // 授权码 5 分钟
        1800,     // 访问令牌 30 分钟
        604800    // 刷新令牌 7 天
    );
```

### 4. 如何实现单点登录（SSO）？

1. 创建统一的授权服务器
2. 多个应用注册为 OAuth2 客户端
3. 用户在授权服务器登录一次
4. 各应用通过 OAuth2 流程获取用户信息

### 5. 支持其他 OAuth2 授权模式吗？

当前仅实现授权码模式。其他模式（密码模式、客户端凭据模式、隐式模式）可根据需要扩展。

## 参考资料

- [OAuth 2.0 RFC 6749](https://tools.ietf.org/html/rfc6749)
- [OAuth 2.0 Security Best Practices](https://tools.ietf.org/html/draft-ietf-oauth-security-topics)
- [示例代码](../sa-token-examples/oauth2_example.rs)

## 下一步

- [JWT 指南](./JWT_GUIDE_zh-CN.md)
- [事件监听](./EVENT_LISTENER_zh-CN.md)
- [权限匹配](./PermissionMatching_zh-CN.md)

