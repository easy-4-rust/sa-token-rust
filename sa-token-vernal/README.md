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

`SaTokenComponents` additionally installs a prebuilt `Arc<SaTokenManager>`, its
`VernalSaTokenBridge`, and an immutable `VernalSaTokenPolicy` as one atomic
Vernal component bundle. It also registers `VernalSaTokenInterceptor` as an
early AOP Advisor. The exact Manager, Bridge, and Policy identities are shared
by the container and interceptor. Authentication and authorization can
short-circuit before the handler, and the resulting `WebFailure` is mapped by
Vernal's Axum/Tonic adapters without leaking token, role, permission, or storage
details:

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

HTTP adapters use `Operation(path_template, http_method)`; Tonic uses
`Operation(service_name, method_name)`. Install Vernal's strict AOP adapter
entry (`with_vernal_aop` for Axum or `TonicAopLayer` for Tonic) so the owned
`HttpRequestSnapshot` and request context reach the interceptor.

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

`SaTokenComponents` 还会把预构造的 `Arc<SaTokenManager>`、
`VernalSaTokenBridge` 与不可变 `VernalSaTokenPolicy` 作为一个原子组件包安装到
Vernal，并把 `VernalSaTokenInterceptor` 注册为靠前执行的 AOP Advisor。容器与
拦截器共享完全相同的 Manager、Bridge 和 Policy `Arc`；认证或授权可以在
Handler 前短路，产生的 `WebFailure` 由 Vernal Axum/Tonic Adapter 映射，客户端
不会看到 Token、角色、权限或存储错误细节。Bridge 对 Manager 的依赖仍显式进入
启动期组件图；任一组件标识冲突时，不会留下只注册一半的状态。

HTTP Adapter 的操作身份是 `Operation(path_template, http_method)`，Tonic 则是
`Operation(service_name, method_name)`。应用需要声明操作，并安装 Axum
`with_vernal_aop` 或 Tonic `TonicAopLayer`，使 owned `HttpRequestSnapshot` 与
`RequestContext` 进入认证拦截器。

`VernalSaTokenPolicy` 支持角色、权限各自的 ALL/ANY 规则。角色精确匹配；权限
保持 Sa-Token-Rust 的精确、全局 `*` 与 `orders:*` 前缀通配符语义。受保护的
匿名调用返回 401，已登录但身份不足返回 403，权限数据源失败返回脱敏 500，原始
错误只保留在服务端错误链。声明空要求时按 fail-closed 拒绝。

Vernal API 仍为 `0.0.0-dev`，因此该桥目前保持实验状态且不发布，并把 Git 依赖
固定到已经验证的 Vernal 提交。
