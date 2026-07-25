//! Sa-Token-Rust 消费 Vernal HTTP/Web 合同的真实桥接测试。

use std::{
    cell::RefCell,
    error::Error,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use async_trait::async_trait;
use http::Request;
use sa_token_adapter::SaRequest;
use sa_token_core::{
    PathAuthConfig, SaTokenConfig, SaTokenContext, SaTokenError, SaTokenManager, StpInterface,
};
use sa_token_storage_memory::MemoryStorage;
use sa_token_vernal::{
    SaTokenComponents, VernalSaRequest, VernalSaTokenBridge, VernalSaTokenError,
    VernalSaTokenPolicy,
};
use tokio_util::sync::CancellationToken;
use vernal_aop::{
    InvocationError, InvocationTarget, InvocationValue, LocalInvocationTarget,
    LocalInvocationValue, Operation,
};
use vernal_context::{ApplicationModuleError, VernalApplicationBuilder};
use vernal_http::HttpRequestSnapshot;
use vernal_ioc::ComponentDefinition;
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

/// 用于证明权限数据源失败不会被错误映射成普通 403 的测试数据源。
struct FailingPermissionSource;

#[async_trait]
impl StpInterface for FailingPermissionSource {
    async fn get_permission_list(
        &self,
        _login_id: &str,
        _login_type: &str,
    ) -> Result<Vec<String>, SaTokenError> {
        Err(SaTokenError::StorageError(
            "permission backend unavailable".to_owned(),
        ))
    }

    async fn get_role_list(
        &self,
        _login_id: &str,
        _login_type: &str,
    ) -> Result<Vec<String>, SaTokenError> {
        Ok(vec!["operator".to_owned()])
    }
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
    let Err(duplicate) = components.install(&mut application) else {
        panic!("duplicate security module should be rejected");
    };
    assert!(matches!(
        duplicate,
        ApplicationModuleError::DuplicateName {
            name: "sa-token.security"
        }
    ));

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
    let policy = context
        .container()
        .resolve::<VernalSaTokenPolicy>()
        .expect("authorization policy should resolve");

    assert!(Arc::ptr_eq(&manager, &resolved_manager));
    assert!(Arc::ptr_eq(&manager, bridge.manager()));
    assert!(Arc::ptr_eq(components.bridge(), &bridge));
    assert!(Arc::ptr_eq(components.authorization_policy(), &policy));
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
async fn definition_conflict_rolls_back_both_security_advisors() {
    let manager = manager();
    let operation = Operation::new("protected_handler", "invoke");
    let components = SaTokenComponents::new(Arc::clone(&manager));
    let mut application =
        VernalApplicationBuilder::current().expect("Tokio runtime should be available");
    application.operation(operation.clone());
    application
        .register(ComponentDefinition::shared_arc(Arc::clone(&manager)))
        .expect("pre-existing manager definition");

    let Err(error) = components.install(&mut application) else {
        panic!("duplicate manager must reject the complete security module");
    };
    assert!(matches!(
        error,
        ApplicationModuleError::Definition {
            module: "sa-token.security",
            ..
        }
    ));

    // Definition 冲突发生时，隔离 Registrar 中的 Bridge、Policy、Send Advisor 与
    // Local Advisor 必须一起丢弃，不能让应用在缺少安全组件时生成残缺调用计划。
    let context = application
        .build()
        .expect("pre-existing manager alone should still build");
    assert!(
        context
            .invocation_plans()
            .get(&operation)
            .is_some_and(|plan| plan.is_empty())
    );
    assert!(
        context
            .local_invocation_plans()
            .get(&operation)
            .is_some_and(|plan| plan.is_empty())
    );
    assert!(
        context
            .container()
            .resolve::<VernalSaTokenBridge>()
            .is_err()
    );
    assert!(
        context
            .container()
            .resolve::<VernalSaTokenPolicy>()
            .is_err()
    );
    assert!(Arc::ptr_eq(
        &context
            .container()
            .resolve::<SaTokenManager>()
            .expect("original manager should remain"),
        &manager
    ));
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

    assert!(
        context.local_invocation_plans().get(&operation).is_some(),
        "same component bundle should also compile the Local-AOP advisor"
    );
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

#[tokio::test(flavor = "current_thread")]
async fn component_bundle_scopes_non_send_local_target_future() {
    let manager = manager();
    let token = manager
        .login("local-aop-user")
        .await
        .expect("Sa-Token login");
    manager
        .set_roles("local-aop-user", vec!["operator".to_owned()])
        .await
        .expect("role setup");
    let operation = Operation::new("/actix/orders/{id}", "GET");
    let policy =
        Arc::new(VernalSaTokenPolicy::new().require_all_roles(operation.clone(), ["operator"]));
    let components = SaTokenComponents::new(Arc::clone(&manager)).with_authorization_policy(policy);
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
        RouteMetadata::new("/actix/orders/{id}", "GET", "/actix/orders/{id}"),
        CancellationToken::new(),
    ));
    request_context
        .extensions()
        .insert(snapshot(
            "/actix/orders/42",
            Some(&format!("Bearer {}", token.as_str())),
        ))
        .await;
    let scope = Arc::new(WebRequestScope::new(request_context.cancellation().clone()));
    let invocation = HandlerInvocation::new(Arc::clone(&request_context), scope)
        .aop_invocation()
        .await;

    // Rc/RefCell 同时进入 Future 和返回值，证明该调用没有被伪装成 Send 链。
    let local_events = Rc::new(RefCell::new(Vec::new()));
    let observed_events = Rc::clone(&local_events);
    let target: Rc<LocalInvocationTarget> = Rc::new(move |_invocation| {
        let observed_events = Rc::clone(&observed_events);
        Box::pin(async move {
            observed_events.borrow_mut().push("target");
            tokio::task::yield_now().await;
            let login_id = SaTokenContext::get_current().and_then(|current| current.login_id);
            Ok(Box::new(Rc::new(login_id)) as LocalInvocationValue)
        })
    });

    let value = context
        .local_invocation_plans()
        .get(&operation)
        .expect("compiled Sa-Token Local-AOP advisor")
        .invoke(invocation, target)
        .await
        .expect("authenticated local invocation");
    let login_id = value
        .downcast::<Rc<Option<String>>>()
        .expect("non-Send local login id");
    assert_eq!(login_id.as_deref(), Some("local-aop-user"));
    assert_eq!(*local_events.borrow(), ["target"]);
    assert_eq!(
        request_context
            .principal()
            .await
            .expect("Vernal principal")
            .subject(),
        "local-aop-user"
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

#[tokio::test]
async fn operation_policy_combines_role_permission_and_or_rules_with_wildcards() {
    let manager = manager();
    let token = manager.login("policy-user").await.expect("Sa-Token login");
    manager
        .set_roles("policy-user", vec!["operator".to_owned()])
        .await
        .expect("role setup");
    manager
        .set_permissions("policy-user", vec!["orders:*".to_owned()])
        .await
        .expect("permission setup");

    let operation = Operation::new("/orders/{id}", "PUT");
    let policy = Arc::new(
        VernalSaTokenPolicy::new()
            .require_all_roles(operation.clone(), ["operator"])
            .require_any_role(operation.clone(), ["admin", "operator"])
            .require_all_permissions(operation.clone(), ["orders:read"])
            .require_any_permission(operation.clone(), ["orders:delete", "orders:write"]),
    );
    let components =
        SaTokenComponents::new(Arc::clone(&manager)).with_authorization_policy(Arc::clone(&policy));
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
        RouteMetadata::new("/orders/{id}", "PUT", "/orders/{id}"),
        CancellationToken::new(),
    ));
    request_context
        .extensions()
        .insert(snapshot(
            "/orders/42",
            Some(&format!("Bearer {}", token.as_str())),
        ))
        .await;
    let scope = Arc::new(WebRequestScope::new(request_context.cancellation().clone()));
    let invocation = HandlerInvocation::new(Arc::clone(&request_context), scope)
        .aop_invocation()
        .await;
    let target: Arc<InvocationTarget> =
        Arc::new(|_invocation| Box::pin(async { Ok(Box::new("updated") as InvocationValue) }));

    let result = context
        .invocation_plans()
        .get(&operation)
        .expect("compiled authorization advisor")
        .invoke(invocation, target)
        .await
        .expect("role and wildcard permission should pass");
    assert_eq!(
        result.downcast::<&'static str>().expect("target result"),
        Box::new("updated")
    );
    let resolved_policy = context
        .container()
        .resolve::<VernalSaTokenPolicy>()
        .expect("policy component");
    assert!(Arc::ptr_eq(&policy, &resolved_policy));
    assert!(Arc::ptr_eq(
        components.authorization_policy(),
        &resolved_policy
    ));
    context.close().await.expect("context should close");
}

#[tokio::test]
async fn operation_policy_returns_403_and_never_calls_target_when_identity_is_insufficient() {
    let manager = manager();
    let token = manager.login("limited-user").await.expect("Sa-Token login");
    manager
        .set_roles("limited-user", vec!["operator".to_owned()])
        .await
        .expect("role setup");
    let operation = Operation::new("/admin/reports", "GET");
    let policy = Arc::new(
        VernalSaTokenPolicy::new().require_all_roles(operation.clone(), ["operator", "auditor"]),
    );
    let components = SaTokenComponents::new(manager).with_authorization_policy(Arc::clone(&policy));
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
        RouteMetadata::new("/admin/reports", "GET", "/admin/reports"),
        CancellationToken::new(),
    ));
    request_context
        .extensions()
        .insert(snapshot(
            "/admin/reports",
            Some(&format!("Bearer {}", token.as_str())),
        ))
        .await;
    let scope = Arc::new(WebRequestScope::new(request_context.cancellation().clone()));
    let invocation = HandlerInvocation::new(request_context, scope)
        .aop_invocation()
        .await;
    let target_called = Arc::new(AtomicBool::new(false));
    let observed_target = Arc::clone(&target_called);
    let target: Arc<InvocationTarget> = Arc::new(move |_invocation| {
        observed_target.store(true, Ordering::SeqCst);
        Box::pin(async { Ok(Box::new(()) as InvocationValue) })
    });

    let error = context
        .invocation_plans()
        .get(&operation)
        .expect("compiled authorization advisor")
        .invoke(invocation, target)
        .await
        .expect_err("missing auditor role must fail");
    let InvocationError::Target { source } = error else {
        panic!("authorization rejection must remain a target failure");
    };
    let failure = source
        .downcast::<WebFailure>()
        .expect("framework-neutral Web failure");
    assert_eq!(failure.problem().status(), 403);
    assert_eq!(failure.problem().title(), "Permission is required");
    assert!(!target_called.load(Ordering::SeqCst));
    context.close().await.expect("context should close");
}

#[tokio::test]
async fn operation_policy_requires_identity_even_without_path_auth_configuration() {
    let operation = Operation::new("/billing", "POST");
    let policy =
        Arc::new(VernalSaTokenPolicy::new().require_any_role(operation.clone(), ["billing-admin"]));
    let components = SaTokenComponents::new(manager()).with_authorization_policy(policy);
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
        RouteMetadata::new("/billing", "POST", "/billing"),
        CancellationToken::new(),
    ));
    request_context
        .extensions()
        .insert(snapshot("/billing", None))
        .await;
    let scope = Arc::new(WebRequestScope::new(request_context.cancellation().clone()));
    let invocation = HandlerInvocation::new(request_context, scope)
        .aop_invocation()
        .await;
    let target: Arc<InvocationTarget> =
        Arc::new(|_invocation| Box::pin(async { Ok(Box::new(()) as InvocationValue) }));

    let error = context
        .invocation_plans()
        .get(&operation)
        .expect("compiled authorization advisor")
        .invoke(invocation, target)
        .await
        .expect_err("anonymous protected operation must fail");
    let InvocationError::Target { source } = error else {
        panic!("anonymous rejection must remain a target failure");
    };
    let failure = source
        .downcast::<WebFailure>()
        .expect("framework-neutral Web failure");
    assert_eq!(failure.problem().status(), 401);
    assert_eq!(failure.problem().title(), "Authentication is required");
    context.close().await.expect("context should close");
}

#[tokio::test]
async fn permission_backend_failure_remains_a_safe_500_with_internal_source_chain() {
    let manager = Arc::new(
        SaTokenManager::new(Arc::new(MemoryStorage::new()), SaTokenConfig::default())
            .with_stp_interface(Arc::new(FailingPermissionSource)),
    );
    let token = manager.login("backend-user").await.expect("Sa-Token login");
    let operation = Operation::new("/reports", "GET");
    let policy = Arc::new(
        VernalSaTokenPolicy::new().require_all_permissions(operation.clone(), ["reports:read"]),
    );
    let components = SaTokenComponents::new(manager).with_authorization_policy(Arc::clone(&policy));
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
        RouteMetadata::new("/reports", "GET", "/reports"),
        CancellationToken::new(),
    ));
    request_context
        .extensions()
        .insert(snapshot(
            "/reports",
            Some(&format!("Bearer {}", token.as_str())),
        ))
        .await;
    let scope = Arc::new(WebRequestScope::new(request_context.cancellation().clone()));
    let invocation = HandlerInvocation::new(request_context, scope)
        .aop_invocation()
        .await;
    let target: Arc<InvocationTarget> =
        Arc::new(|_invocation| Box::pin(async { Ok(Box::new(()) as InvocationValue) }));

    let error = context
        .invocation_plans()
        .get(&operation)
        .expect("compiled authorization advisor")
        .invoke(invocation, target)
        .await
        .expect_err("permission backend failure must fail closed");
    let InvocationError::Target { source } = error else {
        panic!("infrastructure failure must remain a target failure");
    };
    let failure = source
        .downcast::<WebFailure>()
        .expect("framework-neutral Web failure");
    assert_eq!(failure.problem().status(), 500);
    assert_eq!(
        failure.problem().title(),
        "Authentication service is unavailable"
    );
    assert!(failure.source().is_some());
    context.close().await.expect("context should close");
}
