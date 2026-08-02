# Framework Integration

[中文文档](/zh/guide/framework-integration.md) | English

sa-token-rust supports 9 web frameworks through dedicated plugin packages. Each plugin provides middleware, extractors, and state management with a consistent API.

## Supported Frameworks

| Framework | Plugin Package | Feature (default) | Binding Crate |
|-----------|---------------|-------------------|---------------|
| **Axum** | `sa-token-plugin-axum` | `axum-08` | (internal `v08`) |
| **Actix-web** | `sa-token-plugin-actix-web` | `v4` | `sa-token-plugin-actix-web-v4` |
| **Poem** | `sa-token-plugin-poem` | (none) | (direct) |
| **Rocket** | `sa-token-plugin-rocket` | `v05` | `sa-token-plugin-rocket-v05` |
| **Warp** | `sa-token-plugin-warp` | (none) | (direct) |
| **Salvo** | `sa-token-plugin-salvo` | `v079` | `sa-token-plugin-salvo-v079` |
| **Tide** | `sa-token-plugin-tide` | (none) | (direct) |
| **Gotham** | `sa-token-plugin-gotham` | `v074` | `sa-token-plugin-gotham-v074` |
| **Ntex** | `sa-token-plugin-ntex` | `v212` | `sa-token-plugin-ntex-v212` |

Quick reference: [Quick Start](./quick-start.md).

### Version-Split Architecture

Facade crates (Actix-web, Rocket, Salvo, Gotham, Ntex) use Cargo features to select the framework major version at compile time. Each facade re-exports from a version-specific binding crate (`*-v4`, `*-v05`, etc.) and shares a `*-core` crate for common logic (state, adapter, error responses).

```toml
# Default: v4 (production-ready)
sa-token-plugin-actix-web = "0.1.14"

# Explicit feature selection
sa-token-plugin-actix-web = { version = "0.1.14", default-features = false, features = ["v4", "redis"] }
```

All plugins provide:
- State management with Builder pattern
- Dual middleware (basic + login-required)
- Three extractors (required, optional, LoginId)
- Request/Response adapters
- Token extraction from Header/Cookie/Query
- Bearer token support
- `SaTokenLayer` for fine-grained route control
- `with_path_auth(state, config)` for path-based auth rules

---

## Axum

```toml
[dependencies]
sa-token-plugin-axum = "0.1.14"
axum = "0.8"
tokio = { version = "1", features = ["full"] }
```

```rust
use axum::{Router, routing::{get, post}};
use sa_token_plugin_axum::{SaTokenState, SaTokenMiddleware, LoginIdExtractor};

let state = SaTokenState::builder()
    .storage(Arc::new(MemoryStorage::new()))
    .build();

let app = Router::new()
    .route("/user/info", get(user_info))
    .layer(SaTokenMiddleware::new(state));
```

---

## Actix-web

```rust
use actix_web::{App, HttpServer, web};
use sa_token_plugin_actix_web::{SaTokenState, SaTokenMiddleware, LoginIdExtractor};

// Initialize Sa-Token
let sa_token_manager = conf::init_sa_token(None)
    .await
    .expect("Sa-Token initialization failed");

// Create Sa-Token state
let sa_token_state = SaTokenState {
    manager: sa_token_manager.clone(),
};

// Create data for application state
let sa_token_data = web::Data::new(sa_token_state.clone());

HttpServer::new(move || {
    App::new()
        // Register middleware
        .wrap(Logger::default())
        .app_data(sa_token_data.clone()) // Inject Sa-Token into application state
        .wrap(SaTokenMiddleware::new(sa_token_state.clone()))

        // Routes
        .route("/api/login", web::post().to(login))
        .route("/api/user/info", web::get().to(user_info))
})
.bind("0.0.0.0:3000")?
.run()
.await

// For a complete example, see sa-token-examples/actix-web-example/
```

---

## Poem

```rust
use poem::{Route, Server};
use sa_token_plugin_poem::{SaTokenState, SaTokenMiddleware, LoginIdExtractor};

let state = SaTokenState::builder()
    .storage(Arc::new(MemoryStorage::new()))
    .build();

let app = Route::new()
    .at("/user/info", poem::get(user_info))
    .with(SaTokenMiddleware::new(state));

Server::new(TcpListener::bind("127.0.0.1:8080"))
    .run(app)
    .await
```

---

## Rocket

```rust
use rocket::{launch, get, routes};
use sa_token_plugin_rocket::{SaTokenState, SaTokenFairing, LoginIdGuard};

#[get("/user/info")]
fn user_info(login_id: LoginIdGuard) -> String {
    format!("User: {}", login_id.0)
}

#[launch]
fn rocket() -> _ {
    let state = SaTokenState::builder()
        .storage(Arc::new(MemoryStorage::new()))
        .build();

    rocket::build()
        .attach(SaTokenFairing::new(state))
        .mount("/", routes![user_info])
}
```

---

## Warp

```rust
use warp::Filter;
use sa_token_plugin_warp::{SaTokenState, sa_token_filter};

let state = SaTokenState::builder()
    .storage(Arc::new(MemoryStorage::new()))
    .build();

let routes = warp::path("user")
    .and(warp::path("info"))
    .and(sa_token_filter(state))
    .map(|token_data| {
        format!("User info")
    });

warp::serve(routes)
    .run(([127, 0, 0, 1], 8080))
    .await;
```

---

## Salvo

```toml
[dependencies]
sa-token-plugin-salvo = "0.1.14"
salvo = "0.79"
```

```rust
use salvo::prelude::*;
use sa_token_plugin_salvo::{SaTokenState, SaTokenLayer, SaCheckLoginMiddleware, LoginIdExtractor};

#[handler]
async fn user_info(login_id: LoginIdExtractor) -> String {
    format!("User: {}", login_id.login_id())
}

#[tokio::main]
async fn main() {
    let state = SaTokenState::builder()
        .storage(Arc::new(MemoryStorage::new()))
        .build();

    let router = Router::new()
        .hoop(SaTokenLayer::new(state.clone()))
        .push(Router::with_path("/api")
            .push(Router::with_path("/user/info").get(user_info))
            .push(Router::with_path("/admin")
                .hoop(SaCheckLoginMiddleware::new(state))
                .get(admin_panel)));

    Server::new(TcpListener::bind("127.0.0.1:8080"))
        .serve(router)
        .await;
}
```

Salvo also provides `SaCheckPermissionMiddleware` and `SaCheckRoleMiddleware` for fine-grained authorization.

---

## Tide

```toml
[dependencies]
sa-token-plugin-tide = "0.1.14"
tide = "0.17"
```

```rust
use tide::Request;
use sa_token_plugin_tide::{SaTokenState, SaTokenLayer, SaCheckLoginMiddleware, LoginIdExtractor};

#[tokio::main]
async fn main() -> tide::Result<()> {
    let state = SaTokenState::builder()
        .storage(Arc::new(MemoryStorage::new()))
        .build();

    let mut app = tide::with_state(state.clone());
    app.with(SaTokenLayer::new(state.clone()));

    app.at("/api/user/info").get(|req: Request<SaTokenState>| async move {
        let login_id = LoginIdExtractor::from_request(&req).unwrap();
        Ok(format!("User: {}", login_id.login_id()))
    });

    app.at("/api/admin/*")
        .with(SaCheckLoginMiddleware::new(state))
        .get(admin_panel);

    app.listen("127.0.0.1:8080").await?;
    Ok(())
}
```

---

## Gotham

```toml
[dependencies]
sa-token-plugin-gotham = "0.1.14"
gotham = "0.7"
```

```rust
use gotham::prelude::*;
use sa_token_plugin_gotham::{SaTokenState, SaTokenMiddleware, SaCheckLoginMiddleware};

fn router(state: SaTokenState) -> Router {
    build_simple_router(|route| {
        route
            .middleware(SaTokenMiddleware::new(state.clone()))
            .get("/user/info")
            .to(user_info);

        route
            .middleware(SaCheckLoginMiddleware::new(state.clone()))
            .get("/admin")
            .to(admin_panel);
    })
}

fn main() {
    let state = SaTokenState::builder()
        .storage(Arc::new(MemoryStorage::new()))
        .build();

    gotham::start("127.0.0.1:8080", router(state));
}
```

---

## Ntex

```toml
[dependencies]
sa-token-plugin-ntex = "0.1.14"
ntex = "2.12"
```

```rust
use ntex::web;
use sa_token_plugin_ntex::{SaTokenState, SaTokenLayer, SaCheckLoginMiddleware, LoginIdExtractor};

#[web::get("/user/info")]
async fn user_info(login_id: LoginIdExtractor) -> String {
    format!("User: {}", login_id.login_id().unwrap_or("unknown"))
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
    let state = SaTokenState::builder()
        .storage(Arc::new(MemoryStorage::new()))
        .build();

    web::HttpServer::new(move || {
        web::App::new()
            .wrap(SaTokenLayer::new(state.clone()))
            .service(user_info)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
```

---

## Common Patterns

### Middleware Types

Each plugin provides a basic middleware/layer and optional check middleware:

| Framework | Basic (validate+inject, no block) | Login-Required (401 on missing) | Permission (403) | Role (403) |
|---|---|---|---|---|
| **Axum** | `SaTokenLayer` | `SaCheckLoginLayer` | `SaCheckPermissionLayer` | — |
| **Actix-web** | `SaTokenLayer` | `SaCheckLoginMiddleware` | — | — |
| **Poem** | `SaTokenMiddleware` | `SaCheckLoginMiddleware` | — | — |
| **Rocket** | `SaTokenFairing` | `SaCheckLoginFairing` | `SaCheckPermissionFairing` | `SaCheckRoleFairing` |
| **Salvo** | `SaTokenLayer` | `SaCheckLoginMiddleware` | `SaCheckPermissionMiddleware` | `SaCheckRoleMiddleware` |
| **Warp** | `sa_token_filter` | `sa_check_login_filter` | — | — |
| **Tide** | `SaTokenLayer` | `SaCheckLoginMiddleware` | `SaCheckPermissionMiddleware` | `SaCheckRoleMiddleware` |
| **Gotham** | `SaTokenMiddleware` | `SaCheckLoginMiddleware` | `SaCheckPermissionMiddleware` | `SaCheckRoleMiddleware` |
| **Ntex** | `SaTokenLayer` | `SaCheckLoginMiddleware` | `SaCheckPermissionMiddleware` | `SaCheckRoleMiddleware` |

### Extractors

Each plugin provides three extractor types:

1. **LoginIdExtractor** (required): Extracts login ID, returns 401 if not authenticated
2. **OptionalLoginIdExtractor** (optional): Extracts login ID if present, returns `None` if not
3. **TokenValueExtractor**: Extracts the raw token value for custom processing

### Token Sources

Tokens are automatically extracted from multiple sources in priority order:

1. **Authorization Header**: `Authorization: Bearer <token>` or `Authorization: <token>`
2. **Cookie**: Configurable cookie name
3. **Query Parameter**: Configurable query parameter name

### Custom Token Name

```rust
let state = SaTokenState::builder()
    .storage(Arc::new(MemoryStorage::new()))
    .token_name("X-Auth-Token")  // Custom header/cookie/query name
    .build();
```

### Layered Middleware (Axum)

```rust
use sa_token_plugin_axum::{SaTokenLayer, SaCheckLoginLayer, SaCheckPermissionLayer};

let state = SaTokenState::builder()
    .storage(Arc::new(MemoryStorage::new()))
    .build();

let app = Router::new()
    // Global: validate token, inject context (does not block unauthenticated)
    .layer(SaTokenLayer::new(state.clone()))
    // Per-route: require login
    .route("/user/info", get(user_info))
    .route_layer(SaCheckLoginLayer::new(state.clone()))
    // Per-route: require specific permission
    .route("/user/delete", post(delete_user))
    .route_layer(SaCheckPermissionLayer::new(state.clone(), "user:delete"));
```

### Path-based Auth Router

```rust
use sa_token_core::router::{PathAuthConfig, run_auth_flow, extract_token};

let config = PathAuthConfig::new()
    .include(vec!["/api/**".to_string()])
    .exclude(vec!["/api/public/**".to_string()]);

// In middleware / layer:
let token_str = extract_token(&req, token_name);
let flow = run_auth_flow(&req, &manager, Some(&config)).await;
if flow.should_reject() {
    return StatusCode::UNAUTHORIZED;
}
flow.run(service.call(req)).await
```
