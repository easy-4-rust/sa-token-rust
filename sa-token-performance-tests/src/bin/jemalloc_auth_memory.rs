use std::sync::Arc;

use sa_token_core::{SaTokenConfig, SaTokenManager, run_auth_flow};
use sa_token_performance_tests::BenchRequest;
use sa_token_storage_memory::MemoryStorage;
use tikv_jemalloc_ctl::{epoch, stats};

#[global_allocator]
static ALLOCATOR: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn sample() -> (usize, usize) {
    epoch::advance().unwrap();
    (
        stats::allocated::read().unwrap(),
        stats::resident::read().unwrap(),
    )
}

fn main() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let manager = SaTokenManager::new(Arc::new(MemoryStorage::new()), SaTokenConfig::default());
    let cycles = env_usize("SA_TOKEN_MEMORY_CYCLES", 10_000);
    let (allocated_before, resident_before) = sample();

    runtime.block_on(async {
        for index in 0..cycles {
            let token = manager
                .login(format!("jemalloc-user-{index}"))
                .await
                .unwrap();
            let request = BenchRequest {
                token: token.as_str().to_string(),
            };
            let result = run_auth_flow(&request, &manager, None).await;
            assert!(result.auth.is_valid);
            drop(result);
            manager.logout(&token).await.unwrap();
        }
    });

    let (allocated_after, resident_after) = sample();
    let allocated_growth = allocated_after.saturating_sub(allocated_before);
    let resident_growth = resident_after.saturating_sub(resident_before);
    println!(
        "{{\"cycles\":{cycles},\"allocated_before\":{allocated_before},\"allocated_after\":{allocated_after},\"allocated_growth\":{allocated_growth},\"resident_before\":{resident_before},\"resident_after\":{resident_after},\"resident_growth\":{resident_growth}}}"
    );
    assert!(
        allocated_growth <= env_usize("SA_TOKEN_JEMALLOC_MAX_ALLOCATED_GROWTH", 6_000_000),
        "jemalloc allocated growth regression: {allocated_growth} bytes"
    );
    assert!(
        resident_growth <= env_usize("SA_TOKEN_JEMALLOC_MAX_RESIDENT_GROWTH", 32_000_000),
        "jemalloc resident growth regression: {resident_growth} bytes"
    );
}
