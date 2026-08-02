# Sa-Token Actix-web 完整示例

这是一个使用 Sa-Token 与 Actix-web 框架集成的完整示例，展示了如何在 Actix-web 应用中实现认证、授权和权限控制。

## 功能特性

- 用户登录/注册
- 基于 token 的身份验证
- 权限和角色管理
- 多种授权方式（登录检查、权限检查、角色检查）
- 支持内存存储和 Redis 存储
- 完整的 API 示例

## 运行方式

```bash
# 进入示例目录
cd examples/actix-web-example

# 使用内存存储运行
./test.sh

# 使用 Redis 存储运行
./test.sh redis

# 开启 StpUtil 演示
./test.sh demo

# 使用 Redis 存储并开启 StpUtil 演示
./test.sh redis-demo
```

## 测试账号

- 管理员：admin / admin123（拥有所有权限）
- 普通用户：user / user123（有限权限）
- 访客：guest / guest123（最少权限）

## API 接口

### 公开接口（无需认证）

- `GET /` - 首页
- `GET /api/health` - 健康检查
- `POST /api/login` - 用户登录
- `POST /api/register` - 用户注册

### 需要登录的接口

- `GET /api/user/info` - 获取用户信息
- `GET /api/user/profile` - 获取用户资料

### 需要特定权限的接口

- `GET /api/user/list` - 获取用户列表（需要 user:list 权限）
- `POST /api/user/delete` - 删除用户（需要 user:delete 权限）

### 需要管理员角色的接口

- `GET /api/admin/panel` - 管理员面板
- `GET /api/admin/stats` - 管理员统计
- `GET /api/permission/list` - 获取权限列表
- `POST /api/permission/add` - 添加权限
- `POST /api/permission/remove` - 移除权限
- `GET /api/role/list` - 获取角色列表

### 需要多个权限的接口

- `POST /api/user/manage` - 管理用户（需要 user:read 和 user:write 权限）

### 演示接口

- `GET /api/demo/stp-util` - StpUtil 功能演示

## 初始化方式

```rust
// 初始化 Sa-Token
let sa_token_manager = conf::init_sa_token(None)
    .await
    .expect("Sa-Token 初始化失败");

// 创建 Sa-Token 状态
let sa_token_state = web::Data::new(SaTokenState {
    manager: sa_token_manager,
});

// 注册 Sa-Token 中间件
App::new()
    .wrap(Logger::default())
    .app_data(sa_token_state.clone()) // 注入 Sa-Token 到应用状态
    .wrap(SaTokenMiddleware::new(sa_token_state.clone()))
```

## Redis 配置

如果需要使用 Redis 存储，可以提供 Redis 配置：

```rust
let redis_config = RedisConfig {
    url: "redis://127.0.0.1:6379".to_string(),
    prefix: Some("sa_token:".to_string()),
};

let sa_token_manager = conf::init_sa_token(Some(&redis_config))
    .await
    .expect("Sa-Token 初始化失败");
```
