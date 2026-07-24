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

Vernal API 仍为 `0.0.0-dev`，因此该桥目前保持实验状态且不发布，并把 Git 依赖
固定到已经验证的 Vernal 提交。
