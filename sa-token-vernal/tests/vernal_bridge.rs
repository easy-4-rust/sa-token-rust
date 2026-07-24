//! Sa-Token-Rust 消费 Vernal HTTP/Web 合同的真实桥接测试。

use std::sync::Arc;

use http::Request;
use sa_token_adapter::SaRequest;
use sa_token_core::{PathAuthConfig, SaTokenConfig, SaTokenContext, SaTokenManager};
use sa_token_storage_memory::MemoryStorage;
use sa_token_vernal::{VernalSaRequest, VernalSaTokenBridge, VernalSaTokenError};
use tokio_util::sync::CancellationToken;
use vernal_http::HttpRequestSnapshot;
use vernal_web::{RequestContext, RouteMetadata};

fn snapshot(uri: &str, authorization: Option<&str>) -> HttpRequestSnapshot {
    let mut builder = Request::builder().method("GET").uri(uri);
    if let Some(authorization) = authorization {
        builder = builder.header("authorization", authorization);
    }
    let request = builder
        .header("cookie", "device=web; tenant=easy-rust")
        .body(())
        .expect("HTTP request");
    HttpRequestSnapshot::capture(&request)
}

fn request_context(path: &str) -> RequestContext {
    RequestContext::new(
        RouteMetadata::new("test_handler", "invoke", path),
        CancellationToken::new(),
    )
}

fn manager() -> Arc<SaTokenManager> {
    Arc::new(SaTokenManager::new(
        Arc::new(MemoryStorage::new()),
        SaTokenConfig::default(),
    ))
}

#[test]
fn request_adapter_preserves_header_cookie_query_path_and_method() {
    let snapshot = snapshot(
        "/orders/42?tenant=easy%20rust",
        Some("Bearer request-token"),
    );
    let request = VernalSaRequest::new(&snapshot);

    assert_eq!(
        request.get_header("Authorization").as_deref(),
        Some("Bearer request-token")
    );
    assert_eq!(request.get_cookie("tenant").as_deref(), Some("easy-rust"));
    assert_eq!(request.get_param("tenant").as_deref(), Some("easy rust"));
    assert_eq!(request.get_path(), "/orders/42");
    assert_eq!(request.get_method(), "GET");
    assert_eq!(request.get_uri(), "/orders/42?tenant=easy%20rust");
}

#[tokio::test]
async fn bridge_sets_vernal_principal_and_scopes_sa_token_context() {
    let manager = manager();
    let token = manager.login("user-42").await.expect("Sa-Token login");
    manager
        .set_roles("user-42", vec!["admin".to_owned(), "operator".to_owned()])
        .await
        .expect("role setup");
    let snapshot = snapshot("/orders/42", Some(&format!("Bearer {}", token.as_str())));
    let context = request_context("/orders/{id}");

    let authentication = VernalSaTokenBridge::new(manager)
        .authenticate(&snapshot, &context)
        .await
        .expect("Vernal authentication");

    assert!(authentication.is_authenticated());
    assert_eq!(authentication.login_id(), Some("user-42"));
    assert_eq!(authentication.token(), Some(&token));
    let principal = context.principal().await.expect("Vernal principal");
    assert_eq!(principal.subject(), "user-42");
    assert!(principal.has_role("admin"));
    assert!(principal.has_role("operator"));

    let scoped_login_id = authentication
        .run(async {
            tokio::task::yield_now().await;
            SaTokenContext::get_current().and_then(|current| current.login_id)
        })
        .await;
    assert_eq!(scoped_login_id.as_deref(), Some("user-42"));
    assert!(SaTokenContext::get_current().is_none());
}

#[tokio::test]
async fn protected_route_rejects_anonymous_request_and_clears_principal() {
    let context = request_context("/protected");
    context
        .set_principal(Some(Arc::new(vernal_web::SecurityPrincipal::new(
            "stale-user",
            ["stale-role"],
        ))))
        .await;
    let bridge = VernalSaTokenBridge::with_path_auth(
        manager(),
        PathAuthConfig::new().include(vec!["/protected/**".to_owned()]),
    );

    let error = match bridge
        .authenticate(&snapshot("/protected/resource", None), &context)
        .await
    {
        Ok(_) => panic!("anonymous request must be rejected"),
        Err(error) => error,
    };

    assert!(matches!(error, VernalSaTokenError::Unauthorized));
    assert_eq!(error.status(), 401);
    assert_eq!(error.safe_message(), "Authentication is required");
    assert!(context.principal().await.is_none());
}
