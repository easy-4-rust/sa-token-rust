use std::sync::Arc;

use sa_token_core::{SaTokenConfig, SaTokenManager, run_auth_flow};
use sa_token_performance_tests::BenchRequest;
use sa_token_storage_memory::MemoryStorage;

#[global_allocator]
static ALLOCATOR: dhat::Alloc = dhat::Alloc;

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn main() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let manager = SaTokenManager::new(Arc::new(MemoryStorage::new()), SaTokenConfig::default());
    let cycles = env_u64("SA_TOKEN_MEMORY_CYCLES", 500);
    let _profiler = dhat::Profiler::builder().testing().build();

    runtime.block_on(async {
        for index in 0..cycles {
            let token = manager.login(format!("dhat-user-{index}")).await.unwrap();
            let request = BenchRequest {
                token: token.as_str().to_string(),
            };
            let result = run_auth_flow(&request, &manager, None).await;
            assert!(result.auth.is_valid);
            drop(result);
            manager.logout(&token).await.unwrap();
        }
    });

    let stats = dhat::HeapStats::get();
    println!(
        "{{\"cycles\":{cycles},\"total_blocks\":{},\"total_bytes\":{},\"current_bytes\":{},\"max_bytes\":{}}}",
        stats.total_blocks, stats.total_bytes, stats.curr_bytes, stats.max_bytes
    );
    assert!(
        stats.total_bytes <= env_u64("SA_TOKEN_DHAT_MAX_TOTAL_BYTES", 16_000_000),
        "DHAT total allocation regression: {} bytes",
        stats.total_bytes
    );
    assert!(
        stats.max_bytes as u64 <= env_u64("SA_TOKEN_DHAT_MAX_PEAK_BYTES", 1_000_000),
        "DHAT peak allocation regression: {} bytes",
        stats.max_bytes
    );
    assert!(
        stats.curr_bytes as u64 <= env_u64("SA_TOKEN_DHAT_MAX_LIVE_BYTES", 512_000),
        "DHAT live allocation regression: {} bytes",
        stats.curr_bytes
    );
}
