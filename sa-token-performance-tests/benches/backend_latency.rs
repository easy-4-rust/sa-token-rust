use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use criterion::{BenchmarkGroup, Criterion, Throughput, criterion_group, criterion_main};
use sa_token_adapter::SaStorage;
use sa_token_core::{SaTokenConfig, SaTokenManager, run_auth_flow};
use sa_token_performance_tests::BenchRequest;
use sa_token_storage_database::DatabaseStorage;
use sa_token_storage_redis::RedisStorage;

fn unique_prefix(backend: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("sa-bench:{backend}:{nanos}:")
}

fn bench_backend(
    group: &mut BenchmarkGroup<'_, criterion::measurement::WallTime>,
    runtime: &tokio::runtime::Runtime,
    backend: &'static str,
    storage: Arc<dyn SaStorage>,
) {
    let key = format!("{}hot-key", unique_prefix(backend));
    runtime
        .block_on(storage.set(&key, "hot-value", None))
        .unwrap();

    group.bench_function(format!("{backend}/storage_get"), |bencher| {
        bencher.to_async(runtime).iter(|| {
            let storage = Arc::clone(&storage);
            let key = key.clone();
            async move {
                let value = storage.get(&key).await.unwrap();
                std::hint::black_box(value);
            }
        })
    });

    group.bench_function(format!("{backend}/storage_set"), |bencher| {
        bencher.to_async(runtime).iter(|| {
            let storage = Arc::clone(&storage);
            let key = key.clone();
            async move {
                storage.set(&key, "hot-value", None).await.unwrap();
            }
        })
    });

    let config = SaTokenConfig {
        storage_key_prefix: unique_prefix(backend),
        ..SaTokenConfig::default()
    };
    let manager = Arc::new(SaTokenManager::new(Arc::clone(&storage), config));
    let token = runtime
        .block_on(manager.login("backend-benchmark-user"))
        .unwrap();
    let request = BenchRequest {
        token: token.as_str().to_string(),
    };

    group.bench_function(format!("{backend}/valid_auth_flow"), |bencher| {
        bencher.to_async(runtime).iter(|| {
            let manager = Arc::clone(&manager);
            let request = request.clone();
            async move {
                let result = run_auth_flow(&request, &manager, None).await;
                assert!(result.auth.is_valid);
                std::hint::black_box(result);
            }
        })
    });

    let sequence = Arc::new(AtomicU64::new(0));
    group.bench_function(format!("{backend}/login_logout"), |bencher| {
        bencher.to_async(runtime).iter(|| {
            let manager = Arc::clone(&manager);
            let sequence = Arc::clone(&sequence);
            async move {
                let login_id = format!("backend-user-{}", sequence.fetch_add(1, Ordering::Relaxed));
                let token = manager.login(login_id).await.unwrap();
                manager.logout(&token).await.unwrap();
            }
        })
    });
}

fn backend_latency(criterion: &mut Criterion) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let mut backends: Vec<(&'static str, Arc<dyn SaStorage>)> = Vec::new();

    if let Ok(url) = std::env::var("SA_TOKEN_BENCH_REDIS_URL") {
        let storage = runtime
            .block_on(RedisStorage::new(&url, unique_prefix("redis-driver")))
            .expect("connect to real Redis benchmark service");
        backends.push(("redis", Arc::new(storage)));
    }

    if let Ok(url) = std::env::var("SA_TOKEN_BENCH_DATABASE_URL") {
        let storage = runtime
            .block_on(DatabaseStorage::new(&url))
            .expect("connect to real PostgreSQL benchmark service");
        backends.push(("postgres", Arc::new(storage)));
    }

    assert!(
        !backends.is_empty(),
        "set SA_TOKEN_BENCH_REDIS_URL and/or SA_TOKEN_BENCH_DATABASE_URL; this benchmark never substitutes a mock backend"
    );

    let mut group = criterion.benchmark_group("real_backend_network_latency");
    group.throughput(Throughput::Elements(1));
    for (name, storage) in backends {
        bench_backend(&mut group, &runtime, name, storage);
    }
    group.finish();
}

criterion_group!(benches, backend_latency);
criterion_main!(benches);
