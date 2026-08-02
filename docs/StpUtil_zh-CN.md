# StpUtil API 参考

中文文档 | [English](StpUtil.md)

`StpUtil` 是一个工具类，为常见的认证和授权操作提供简化的静态 API。它将 `SaTokenManager` 功能封装在易用的接口中。

## 目录

- [初始化](#初始化)
- [登录操作](#登录操作)
- [登出操作](#登出操作)
- [Token 验证](#token-验证)
- [Session 管理](#session-管理)
- [权限管理](#权限管理)
- [角色管理](#角色管理)
- [高级用法](#高级用法)

## 初始化

当您使用任何 Web 框架插件创建 `SaTokenState` 时，`StpUtil` 会自动初始化：

```rust
use sa_token_core::StpUtil;
use sa_token_plugin_axum::SaTokenState;  // 或其他框架插件
use sa_token_storage_memory::MemoryStorage;
use std::sync::Arc;

// 构建状态时 StpUtil 会自动初始化
let state = SaTokenState::builder()
    .storage(Arc::new(MemoryStorage::new()))
    .token_name("Authorization")
    .timeout(86400)
    .build();

// StpUtil 已就绪，可以直接使用！
StpUtil::login("user_id").await?;
```

**注意**：初始化在 `SaTokenState::builder().build()` 中自动完成，所以您无需手动调用任何初始化方法。这适用于所有支持的 Web 框架（Axum、Actix-web、Poem、Rocket、Warp）。

## 登录操作

### 基本登录

```rust
use sa_token_core::StpUtil;

// 使用字符串 ID 登录
let token = StpUtil::login("user_10001").await?;
println!("生成的 token: {}", token.value());

// 使用数字 ID 登录
let token = StpUtil::login(10001).await?;  // 支持 i32, i64, u32, u64
```

### 链式构建登录

```rust
use sa_token_core::StpUtil;
use serde_json::json;

// 链式调用设置额外信息
let token = StpUtil::builder("user_123")
    .extra_data(json!({"ip": "192.168.1.1"}))
    .device("pc")
    .login_type("admin")
    .login(None) // None 表示使用构建器中的 login_id，Some("other_id") 可覆盖
    .await?;

let token = StpUtil::builder("user_123")
    .extra_data(json!({"ip": "192.168.1.1"}))
    .device("pc")
    .login_type("admin")
    .login(Some("new_user_456"))  // 或 Some(10001) 数字ID
    .await?;

```

### 带设备标识的登录

```rust
// 带设备信息登录（用于多设备管理）
let token = StpUtil::login_by_device("user_10001", "mobile_ios").await?;
```

## 登出操作

### 登出当前用户

```rust
// 通过 login_id 登出
StpUtil::logout("user_10001").await?;

// 通过 token 登出
StpUtil::logout_by_token(&token).await?;
```

### 从特定设备登出

```rust
// 从特定设备登出
StpUtil::logout_by_device("user_10001", "mobile_ios").await?;
```

### 踢用户下线

```rust
// 强制登出（踢下线）
StpUtil::kick_out("user_10001").await?;
```

## Token 验证

### 检查是否登录

```rust
// 检查用户是否已登录
let is_logged_in = StpUtil::is_login("user_10001").await;

if is_logged_in {
    println!("用户已登录");
} else {
    println!("用户未登录");
}
```

### 验证 Token

```rust
use sa_token_core::token::TokenValue;

let token = TokenValue::new("your_token_string".to_string());

// 检查 token 是否有效
let is_valid = StpUtil::is_valid(&token).await;

// 获取 token 信息
let token_info = StpUtil::get_token_info(&token).await?;
println!("登录 ID: {}", token_info.login_id);
println!("设备: {:?}", token_info.device);
```

### 从 Token 获取登录 ID

```rust
// 从 token 获取 login_id
let login_id = StpUtil::get_login_id_by_token(&token).await?;
```

## Session 管理

### 获取 Session

```rust
// 获取用户 session
let session = StpUtil::get_session("user_10001").await?;

// 在 session 中存储数据
session.set("username", "张三".to_string()).await;
session.set("email", "zhangsan@example.com".to_string()).await;

// 从 session 检索数据
let username: Option<String> = session.get("username").await;
println!("用户名: {:?}", username);
```

### Session 操作

```rust
// 检查 session 键是否存在
let exists = session.has("email").await;

// 删除 session 数据
session.remove("email").await;

// 清除所有 session 数据
session.clear().await;

// 获取所有 session 键
let keys = session.keys().await;
```

### 删除 Session

```rust
// 删除用户 session
StpUtil::delete_session("user_10001").await?;
```

## 权限管理

### 设置权限

```rust
// 设置用户权限
StpUtil::set_permissions(
    "user_10001",
    vec![
        "user:list".to_string(),
        "user:add".to_string(),
        "user:edit".to_string(),
        "user:delete".to_string(),
    ]
).await?;
```

### 检查权限

```rust
// 检查用户是否有某个权限
let has_permission = StpUtil::has_permission("user_10001", "user:delete").await;

if has_permission {
    println!("用户可以删除");
} else {
    println!("用户不能删除");
}
```

### 检查多个权限

```rust
// 检查用户是否拥有所有权限（AND）
let has_all = StpUtil::has_permissions_and(
    "user_10001",
    &["user:list", "user:add"]
).await;

// 检查用户是否拥有任一权限（OR）
let has_any = StpUtil::has_permissions_or(
    "user_10001",
    &["user:delete", "admin:all"]
).await;
```

### 获取用户权限

```rust
// 获取用户的所有权限
let permissions = StpUtil::get_permissions("user_10001").await;
println!("用户权限: {:?}", permissions);
```

### 清除权限

```rust
// 清除用户的所有权限
StpUtil::clear_permissions("user_10001").await?;
```

## 角色管理

### 设置角色

```rust
// 设置用户角色
StpUtil::set_roles(
    "user_10001",
    vec![
        "user".to_string(),
        "vip".to_string(),
    ]
).await?;

// 设置管理员角色
StpUtil::set_roles(
    "admin_10001",
    vec!["admin".to_string()]
).await?;
```

### 检查角色

```rust
// 检查用户是否有某个角色
let is_admin = StpUtil::has_role("user_10001", "admin").await;

if is_admin {
    println!("用户是管理员");
}
```

### 检查多个角色

```rust
// 检查用户是否拥有所有角色（AND）
let has_all_roles = StpUtil::has_roles_and(
    "user_10001",
    &["user", "vip"]
).await;

// 检查用户是否拥有任一角色（OR）
let has_any_role = StpUtil::has_roles_or(
    "user_10001",
    &["admin", "moderator"]
).await;
```

### 获取用户角色

```rust
// 获取用户的所有角色
let roles = StpUtil::get_roles("user_10001").await;
println!("用户角色: {:?}", roles);
```

### 清除角色

```rust
// 清除用户的所有角色
StpUtil::clear_roles("user_10001").await?;
```

## 高级用法

### 完整登录流程示例

```rust
use sa_token_core::StpUtil;

// 1. 用户登录
let login_id = "user_10001";
let token = StpUtil::login(login_id).await?;

// 2. 设置用户权限
StpUtil::set_permissions(
    login_id,
    vec![
        "user:list".to_string(),
        "user:add".to_string(),
        "post:create".to_string(),
    ]
).await?;

// 3. 设置用户角色
StpUtil::set_roles(
    login_id,
    vec!["user".to_string(), "author".to_string()]
).await?;

// 4. 在 session 中存储额外数据
let session = StpUtil::get_session(login_id).await?;
session.set("username", "张三".to_string()).await;
session.set("email", "zhangsan@example.com".to_string()).await;
session.set("last_login", chrono::Utc::now().to_string()).await;

// 返回 token 给客户端
Ok(token.value().to_string())
```

### 中间件中的 Token 验证

```rust
use sa_token_core::StpUtil;
use sa_token_core::token::TokenValue;

async fn validate_request(token_string: &str) -> Result<String, String> {
    let token = TokenValue::new(token_string.to_string());
    
    // 验证 token
    if !StpUtil::is_valid(&token).await {
        return Err("无效的 token".to_string());
    }
    
    // 获取 login_id
    let login_id = StpUtil::get_login_id_by_token(&token).await
        .map_err(|_| "无法获取 login_id".to_string())?;
    
    // 检查用户是否仍然登录
    if !StpUtil::is_login(&login_id).await {
        return Err("用户未登录".to_string());
    }
    
    Ok(login_id)
}
```

### 基于权限的访问控制

```rust
use sa_token_core::StpUtil;

async fn delete_user(operator_id: &str, target_user_id: &str) -> Result<(), String> {
    // 检查操作者是否有删除权限
    if !StpUtil::has_permission(operator_id, "user:delete").await {
        return Err("无权删除用户".to_string());
    }
    
    // 额外检查：管理员可以删除任何人，普通用户只能删除自己
    let is_admin = StpUtil::has_role(operator_id, "admin").await;
    
    if !is_admin && operator_id != target_user_id {
        return Err("只能删除自己的账户".to_string());
    }
    
    // 执行删除
    // ... 你的删除逻辑
    
    Ok(())
}
```

### 多设备 Session 管理

```rust
use sa_token_core::StpUtil;

// 用户从不同设备登录
let token_web = StpUtil::login_by_device("user_10001", "web").await?;
let token_mobile = StpUtil::login_by_device("user_10001", "mobile_ios").await?;
let token_app = StpUtil::login_by_device("user_10001", "desktop_app").await?;

// 从特定设备登出
StpUtil::logout_by_device("user_10001", "mobile_ios").await?;

// 用户在其他设备上仍然登录
assert!(StpUtil::is_login("user_10001").await);

// 从所有设备登出
StpUtil::logout("user_10001").await?;
```

### 使用泛型类型

```rust
use sa_token_core::StpUtil;

// StpUtil 支持任何实现 Display 的类型
// 包括：String, &str, i32, i64, u32, u64 等

// String login_id
let token1 = StpUtil::login("user_string".to_string()).await?;

// &str login_id
let token2 = StpUtil::login("user_str").await?;

// 数字 login_id
let token3 = StpUtil::login(10001_i32).await?;
let token4 = StpUtil::login(20001_i64).await?;
let token5 = StpUtil::login(30001_u32).await?;

// 所有方法都接受泛型类型
StpUtil::set_permissions(10001, vec!["user:list".to_string()]).await?;
StpUtil::has_role(20001_i64, "admin").await;
let session = StpUtil::get_session(30001_u32).await?;
```

## 错误处理

所有 `StpUtil` 方法都返回 `Result` 类型。适当处理错误：

```rust
use sa_token_core::StpUtil;

match StpUtil::login("user_10001").await {
    Ok(token) => {
        println!("登录成功: {}", token.value());
    }
    Err(e) => {
        eprintln!("登录失败: {:?}", e);
    }
}

// 或使用 ? 操作符
let token = StpUtil::login("user_10001").await?;
```

## 最佳实践

1. **自动初始化**：`StpUtil` 在构建 `SaTokenState` 时自动初始化，无需手动初始化。

2. **错误处理**：始终适当处理 `StpUtil` 方法的错误。

3. **权限命名**：使用一致的权限命名规范（例如 `resource:action`）。

4. **角色层次**：设计清晰的角色层次结构（例如 admin > moderator > user）。

5. **Session 数据**：在 session 中存储最少的、非敏感的数据。

6. **安全事件时登出**：当发生安全敏感事件（密码更改等）时，始终调用 `logout` 或 `kick_out`。

7. **Token 验证**：在处理请求前始终验证 token。

8. **泛型类型**：利用泛型 `LoginId` 支持，使用不同 ID 类型编写更清晰的代码。

## 另见

- [主文档](../README_zh-CN.md)
- [示例](../sa-token-examples/)
- [Web 框架集成](../README_zh-CN.md#框架集成示例)

