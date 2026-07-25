use criterion::{Criterion, criterion_group, criterion_main};
use sa_token_core::{TempJwtTemplate, Totp, base32};
use serde_json::json;

fn core_hot_paths(criterion: &mut Criterion) {
    let payload = b"sa-token-rust-observability-benchmark-payload";
    criterion.bench_function("base32_encode", |bencher| {
        bencher.iter(|| base32::encode(std::hint::black_box(payload)))
    });

    let encoded = base32::encode(payload);
    criterion.bench_function("base32_decode", |bencher| {
        bencher.iter(|| base32::decode(std::hint::black_box(&encoded)).unwrap())
    });

    let totp = Totp::default();
    let secret = base32::encode(b"01234567890123456789");
    criterion.bench_function("totp_generate_at", |bencher| {
        bencher.iter(|| {
            totp.generate_at(std::hint::black_box(&secret), 1_700_000_000)
                .unwrap()
        })
    });

    let template = TempJwtTemplate::new("benchmark-secret-at-least-32-bytes").unwrap();
    let value = json!({"subject": "benchmark", "scope": ["read", "write"]});
    criterion.bench_function("temp_jwt_create_and_parse", |bencher| {
        bencher.iter(|| {
            let token = template
                .create_token(std::hint::black_box(value.clone()), 60)
                .unwrap();
            template.parse_token(&token).unwrap()
        })
    });
}

criterion_group!(benches, core_hot_paths);
criterion_main!(benches);
