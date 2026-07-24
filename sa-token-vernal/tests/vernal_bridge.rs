//! Sa-Token-Rust 消费 Vernal HTTP/Web 合同的真实桥接测试。

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use http::Request;
use sa_token_adapter::SaRequest;
use sa_token_core::{PathAuthConfig, SaTokenConfig, SaTokenContext, SaTokenManager};
use sa_token_storage_memory::MemoryStorage;
use sa_token_vernal::{
    SaTokenComponents, VernalSaRequest, VernalSaTokenBridge, VernalSaTokenError,
};
use tokio_util::sync::CancellationToken;
use vernal_aop::{InvocationError, InvocationTarget, InvocationValue, Operation};
use vernal_context::VernalApplicationBuilder;
use vernal_http::HttpRequestSnapshot;
use vernal_web::{HandlerInvocation, RequestContext, RouteMetadata, WebFailure, WebRequestScope};

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

#[tokio::test]
async fn component_bundle_preserves_manager_identity_and_rejects_duplicates_atomically() {
    let manager = manager();
    let components = SaTokenComponents::new(Arc::clone(&manager))
        .with_path_auth(PathAuthConfig::new().include(vec!["/protected/**".to_owned()]));
    let mut application =
        VernalApplicationBuilder::current().expect("Tokio runtime should be available");
    components
        .install(&mut application)
        .expect("Sa-Token components should install");
    assert!(
        components.install(&mut application).is_err(),
        "duplicate bundle should be rejected without replacing the first install"
    );

    let context = application.build().expect("application should build");
    context.refresh().await.expect("context should refresh");
    context.start().await.expect("context should start");
    let resolved_manager = context
        .container()
        .resolve::<SaTokenManager>()
        .expect("manager should resolve");
    let bridge = context
        .container()
        .resolve::<VernalSaTokenBridge>()
        .expect("bridge should resolve");

    assert!(Arc::ptr_eq(&manager, &resolved_manager));
    assert!(Arc::ptr_eq(&manager, bridge.manager()));
    assert!(Arc::ptr_eq(components.bridge(), &bridge));
    assert!(matches!(
        bridge
            .authenticate(
                &snapshot("/protected/resource", None),
                &request_context("/protected/**"),
            )
            .await,
        Err(VernalSaTokenError::Unauthorized)
    ));

    context.close().await.expect("context should close");
}

#[tokio::test]
async fn component_bundle_compiles_authentication_advisor_and_scopes_target_future() {
    let manager = manager();
    let token = manager.login("aop-user").await.expect("Sa-Token login");
    manager
        .set_roles("aop-user", vec!["operator".to_owned()])
        .await
        .expect("role setup");
    let components = SaTokenComponents::new(Arc::clone(&manager));
    let operation = Operation::new("order_handler", "create");
    let mut application =
        VernalApplicationBuilder::current().expect("Tokio runtime should be available");
    application.operation(operation.clone());
    components
        .install(&mut application)
        .expect("Sa-Token components should install");
    let context = application.build().expect("application should build");
    context.refresh().await.expect("context should refresh");
    context.start().await.expect("context should start");

    let request_context = Arc::new(RequestContext::new(
        RouteMetadata::new("order_handler", "create", "/orders"),
        CancellationToken::new(),
    ));
    request_context
        .extensions()
        .insert(snapshot(
            "/orders",
            Some(&format!("Bearer {}", token.as_str())),
        ))
        .await;
    let scope = Arc::new(WebRequestScope::new(request_context.cancellation().clone()));
    let invocation = HandlerInvocation::new(Arc::clone(&request_context), scope)
        .aop_invocation()
        .await;
    let target: Arc<InvocationTarget> = Arc::new(|_invocation| {
        Box::pin(async {
            let login_id = SaTokenContext::get_current().and_then(|current| current.login_id);
            Ok(Box::new(login_id) as InvocationValue)
        })
    });

    let value = context
        .invocation_plans()
        .get(&operation)
        .expect("compiled Sa-Token advisor")
        .invoke(invocation, target)
        .await
        .expect("authenticated invocation");
    let login_id = value.downcast::<Option<String>>().expect("target login id");
    assert_eq!(login_id.as_deref(), Some("aop-user"));
    assert_eq!(
        request_context
            .principal()
            .await
            .expect("Vernal principal")
            .subject(),
        "aop-user"
    );
    assert!(SaTokenContext::get_current().is_none());
    context.close().await.expect("context should close");
}

#[tokio::test]
async fn authentication_advisor_short_circuits_protected_operation_with_web_failure() {
    let target_called = Arc::new(AtomicBool::new(false));
    let components = SaTokenComponents::new(manager())
        .with_path_auth(PathAuthConfig::new().include(vec!["/protected/**".to_owned()]));
    let operation = Operation::new("protected_handler", "read");
    let mut application =
        VernalApplicationBuilder::current().expect("Tokio runtime should be available");
    application.operation(operation.clone());
    components
        .install(&mut application)
        .expect("Sa-Token components should install");
    let context = application.build().expect("application should build");
    context.refresh().await.expect("context should refresh");
    context.start().await.expect("context should start");

    let request_context = Arc::new(RequestContext::new(
        RouteMetadata::new("protected_handler", "read", "/protected/{resource}"),
        CancellationToken::new(),
    ));
    request_context
        .extensions()
        .insert(snapshot("/protected/resource", None))
        .await;
    let scope = Arc::new(WebRequestScope::new(request_context.cancellation().clone()));
    let invocation = HandlerInvocation::new(Arc::clone(&request_context), scope)
        .aop_invocation()
        .await;
    let observed_target = Arc::clone(&target_called);
    let target: Arc<InvocationTarget> = Arc::new(move |_invocation| {
        observed_target.store(true, Ordering::SeqCst);
        Box::pin(async { Ok(Box::new(()) as InvocationValue) })
    });

    let error = context
        .invocation_plans()
        .get(&operation)
        .expect("compiled Sa-Token advisor")
        .invoke(invocation, target)
        .await
        .expect_err("anonymous protected operation must fail");
    let InvocationError::Target { source } = error else {
        panic!("security rejection must remain a target failure");
    };
    let failure = source
        .downcast::<WebFailure>()
        .expect("framework-neutral Web failure");
    assert_eq!(failure.problem().status(), 401);
    assert_eq!(failure.problem().title(), "Authentication is required");
    assert!(!target_called.load(Ordering::SeqCst));
    assert!(request_context.principal().await.is_none());
    context.close().await.expect("context should close");
}
