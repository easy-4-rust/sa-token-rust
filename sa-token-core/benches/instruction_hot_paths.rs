use std::collections::HashMap;
use std::sync::Arc;

use gungraun::{
    Callgrind, Dhat, DhatMetric, EventKind, LibraryBenchmarkConfig, library_benchmark,
    library_benchmark_group, main,
};
use sa_token_adapter::SaRequest;
use sa_token_core::{SaTokenConfig, SaTokenManager, Totp, base32, run_auth_flow};
use sa_token_storage_memory::MemoryStorage;

#[library_benchmark]
fn base32_round_trip() {
    let payload = b"sa-token-rust-instruction-regression-payload";
    let encoded = base32::encode(std::hint::black_box(payload));
    std::hint::black_box(base32::decode(&encoded).unwrap());
}

#[library_benchmark]
fn totp_generate_at() {
    let totp = Totp::default();
    let secret = base32::encode(b"01234567890123456789");
    std::hint::black_box(totp.generate_at(&secret, 1_700_000_000).unwrap());
}

struct BenchRequest(String);

impl SaRequest for BenchRequest {
    fn get_header(&self, name: &str) -> Option<String> {
        name.eq_ignore_ascii_case("satoken").then(|| self.0.clone())
    }

    fn get_cookie(&self, _: &str) -> Option<String> {
        None
    }

    fn get_param(&self, _: &str) -> Option<String> {
        None
    }

    fn get_headers(&self) -> HashMap<String, String> {
        HashMap::from([("satoken".to_string(), self.0.clone())])
    }

    fn get_path(&self) -> String {
        "/api/orders/42".to_string()
    }

    fn get_method(&self) -> String {
        "GET".to_string()
    }
}

#[library_benchmark]
fn full_login_auth_logout() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let manager = SaTokenManager::new(Arc::new(MemoryStorage::new()), SaTokenConfig::default());
        let token = manager.login("instruction-user").await.unwrap();
        let request = BenchRequest(token.as_str().to_string());
        let result = run_auth_flow(&request, &manager, None).await;
        assert!(result.auth.is_valid);
        manager.logout(&token).await.unwrap();
    });
}

library_benchmark_group!(
    name = hot_paths;
    benchmarks = base32_round_trip, totp_generate_at, full_login_auth_logout
);
main!(
    config = LibraryBenchmarkConfig::default()
        .tool(Callgrind::default().soft_limits([(EventKind::Ir, 7.5)]))
        .tool(
            Dhat::default()
                .soft_limits([(DhatMetric::TotalBytes, 10.0)])
                .hard_limits([(DhatMetric::TotalBytes, 64_000_000)])
        ),
    library_benchmark_groups = hot_paths
);
