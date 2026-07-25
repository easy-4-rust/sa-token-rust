use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use sa_token_adapter::SaRequest;
use sa_token_core::{PathAuthConfig, SaTokenConfig, SaTokenManager, run_auth_flow};
use sa_token_storage_memory::MemoryStorage;

#[derive(Clone)]
struct BenchRequest {
    token: String,
}

impl SaRequest for BenchRequest {
    fn get_header(&self, name: &str) -> Option<String> {
        name.eq_ignore_ascii_case("satoken")
            .then(|| self.token.clone())
    }

    fn get_cookie(&self, _: &str) -> Option<String> {
        None
    }

    fn get_param(&self, _: &str) -> Option<String> {
        None
    }

    fn get_headers(&self) -> HashMap<String, String> {
        HashMap::from([("satoken".to_string(), self.token.clone())])
    }

    fn get_path(&self) -> String {
        "/api/orders/42".to_string()
    }

    fn get_method(&self) -> String {
        "GET".to_string()
    }
}

fn auth_throughput(criterion: &mut Criterion) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let manager = Arc::new(SaTokenManager::new(
        Arc::new(MemoryStorage::new()),
        SaTokenConfig::default(),
    ));
    let token = runtime.block_on(manager.login("benchmark-user")).unwrap();
    let valid_request = BenchRequest {
        token: token.as_str().to_string(),
    };
    let invalid_request = BenchRequest {
        token: "invalid-benchmark-token".to_string(),
    };
    let path_config = PathAuthConfig::new().include(vec!["/api/**".to_string()]);

    let mut group = criterion.benchmark_group("auth_lifecycle_memory");
    group.throughput(Throughput::Elements(1));

    group.bench_function("valid_auth_flow", |bencher| {
        bencher.to_async(&runtime).iter(|| async {
            let result = run_auth_flow(&valid_request, &manager, Some(&path_config)).await;
            assert!(result.auth.is_valid);
            std::hint::black_box(result);
        })
    });

    group.bench_function("rejected_auth_flow", |bencher| {
        bencher.to_async(&runtime).iter(|| async {
            let result = run_auth_flow(&invalid_request, &manager, Some(&path_config)).await;
            assert!(result.auth.should_reject());
            std::hint::black_box(result);
        })
    });

    let sequence = AtomicU64::new(0);
    group.bench_function("login_then_logout", |bencher| {
        bencher.to_async(&runtime).iter(|| {
            let login_id = format!(
                "benchmark-user-{}",
                sequence.fetch_add(1, Ordering::Relaxed)
            );
            let manager = Arc::clone(&manager);
            async move {
                let token = manager.login(login_id).await.unwrap();
                manager.logout(&token).await.unwrap();
            }
        })
    });
    group.finish();
}

criterion_group!(benches, auth_throughput);
criterion_main!(benches);
