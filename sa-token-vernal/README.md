# sa-token-vernal

`sa-token-vernal` is the consumer-owned bridge from Sa-Token-Rust to
[句芒 · Vernal](https://github.com/easy-4-rust/vernal).

It keeps the ownership boundary explicit:

- Sa-Token-Rust owns tokens, sessions, path authentication, roles, and
  authorization semantics.
- Vernal owns IoC, AOP, application context, request context, and framework
  lifecycle integration.
- This crate adapts `HttpRequestSnapshot` to `SaRequest`, runs the shared
  Sa-Token authentication flow, and projects an authenticated identity into
  Vernal's read-only `SecurityPrincipal`.

The returned `VernalAuthentication` owns the request-level `SaTokenContext`.
Run the downstream future through `VernalAuthentication::run` so `StpUtil`
continues to see the correct request identity across `.await` and Tokio worker
switches.

`SaTokenComponents` is the named `sa-token.security` `ApplicationModule`. It
installs a prebuilt `Arc<SaTokenManager>`, its `VernalSaTokenBridge`, an
immutable `VernalSaTokenPolicy`, and both security Advisors as one atomic
Vernal transaction. The exact Manager, Bridge, and Policy identities are shared
by the container and both execution planes. A duplicate module or component
definition rejects the complete transaction without leaving either a Send-AOP
or Local-AOP plan behind.
Authentication and authorization can short-circuit before the handler, and the
resulting `WebFailure` is mapped by Vernal's Axum, Poem, Tonic, and Actix
adapters without leaking token, role, permission, or storage details:

```rust
let operation = Operation::new("/orders/{id}", "PUT");
let policy = Arc::new(
    VernalSaTokenPolicy::new()
        .require_any_role(operation.clone(), ["admin", "operator"])
        .require_all_permissions(operation.clone(), ["orders:write"]),
);
let components = SaTokenComponents::new(manager)
    .with_path_auth(path_auth)
    .with_authorization_policy(policy);
let mut application = VernalApplicationBuilder::current()?;
application.operation(operation);
components.install(&mut application)?;
let context = application.build()?;
```

`VernalSaTokenConfigBinder` maps the immutable Vernal
`ApplicationEnvironment` into Sa-Token's native `SaTokenConfigBuilder`.
Properties use the `sa-token.*` prefix by default, for example
`sa-token.timeout=7200` and `sa-token.token-style=random-64`. Missing properties
keep Sa-Token's own defaults. Storage, listeners, manager construction, and
runtime installation remain explicit Sa-Token responsibilities; the binder
never creates process-global state and never includes secret values in errors.

HTTP adapters use `Operation(path_template, http_method)`; Tonic uses
`Operation(service_name, method_name)`. Install the framework's strict AOP
entry so the owned `HttpRequestSnapshot` and request context reach the
interceptor. Send-capable adapters consume the Send plan; Actix wraps a
concrete `web::resource(...)` with
`VernalActixMiddleware::strict_aop(...)` and consumes the Local plan.

`VernalSaTokenPolicy` supports all/any role rules and all/any permission rules.
Roles use exact matching. Permissions preserve Sa-Token-Rust's exact, global
`*`, and prefix wildcard (`orders:*`) semantics. A protected anonymous call
returns 401, an authenticated but insufficient identity returns 403, and a
permission backend failure remains a safe 500 with its internal source chain
available only to server-side diagnostics. Empty declared requirements fail
closed.

This bridge is experimental and `publish = false` while Vernal's API is
`0.0.0-dev`. Its Git dependency is pinned to a verified Vernal commit.

---

`sa-token-vernal` 是 Sa-Token-Rust 消费
[句芒 · Vernal](https://github.com/easy-4-rust/vernal) 的桥接 crate。

它保持清晰的责任边界：

- Sa-Token-Rust 拥有 Token、Session、路径鉴权、角色和授权语义；
- Vernal 拥有 IoC、AOP、应用上下文、请求上下文和框架生命周期集成；
- 本 crate 将 `HttpRequestSnapshot` 适配为 `SaRequest`，复用统一认证流，并把
  已认证身份投影到 Vernal 的只读 `SecurityPrincipal`。

返回的 `VernalAuthentication` 持有请求级 `SaTokenContext`。业务 Future 应通过
`VernalAuthentication::run` 执行，以保证 `StpUtil` 跨 `.await` 和 Tokio Worker
切换后仍读取当前请求身份。

`SaTokenComponents` 是具名 `sa-token.security` `ApplicationModule`，会把预构造的
`Arc<SaTokenManager>`、`VernalSaTokenBridge`、不可变
`VernalSaTokenPolicy`、Send Advisor 与 Local Advisor 作为一个事务安装到
Vernal。容器与拦截器共享完全相同的 Manager、Bridge 和 Policy `Arc`；支持 Send
的 Adapter 和 Actix 的 `Rc`、非 `Send` Service Future 因而使用完全一致的安全
语义。模块或任一组件标识冲突时，完整事务被拒绝，不会遗留任何残缺组件或安全
AOP 计划。认证或授权可以在 Handler 前短路，产生的 `WebFailure` 由 Vernal
Axum、Poem、Tonic 与 Actix Adapter 映射，客户端不会看到 Token、角色、权限或
存储错误细节。Bridge 对 Manager 的依赖仍显式进入启动期组件图。

HTTP Adapter 的操作身份是 `Operation(path_template, http_method)`，Tonic 则是
`Operation(service_name, method_name)`。应用需要声明操作并安装相应严格 AOP
入口，使 owned `HttpRequestSnapshot` 与 `RequestContext` 进入认证拦截器；
支持 Send 的 Adapter 消费 Send 计划，Actix 则在具体 `web::resource(...)` 上安装
`VernalActixMiddleware::strict_aop(...)` 并消费 Local 计划。

`VernalSaTokenPolicy` 支持角色、权限各自的 ALL/ANY 规则。角色精确匹配；权限
保持 Sa-Token-Rust 的精确、全局 `*` 与 `orders:*` 前缀通配符语义。受保护的
匿名调用返回 401，已登录但身份不足返回 403，权限数据源失败返回脱敏 500，原始
错误只保留在服务端错误链。声明空要求时按 fail-closed 拒绝。

`VernalSaTokenConfigBinder` 会把不可变的 Vernal `ApplicationEnvironment`
映射到 Sa-Token 原生 `SaTokenConfigBuilder`。默认键前缀为 `sa-token.*`，例如
`sa-token.timeout=7200` 和 `sa-token.token-style=random-64`；未声明的字段继续
使用 Sa-Token 自己的默认值。Storage、Listener、Manager 构造和 Runtime 安装仍由
Sa-Token 显式负责，绑定器不会创建进程级全局状态，也不会在错误中输出密钥值。

Vernal API 仍为 `0.0.0-dev`，因此该桥目前保持实验状态且不发布，并把 Git 依赖
固定到已经验证的 Vernal 提交。
