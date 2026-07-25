use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use sa_token_adapter::SaRequest;
use sa_token_plugin_actix_web_v4::ActixRequestAdapter;
use sa_token_plugin_axum::AxumRequestAdapter;
use sa_token_plugin_poem::PoemRequestAdapter;
use sa_token_plugin_tonic::TonicCapturedRequest;

fn exercise<R: SaRequest>(request: &R) -> usize {
    request.get_header("satoken").map_or(0, |value| value.len())
        + request.get_cookie("session").map_or(0, |value| value.len())
        + request.get_param("page").map_or(0, |value| value.len())
        + request.get_path().len()
        + request.get_method().len()
}

fn framework_adapter_comparison(criterion: &mut Criterion) {
    let uri = "/api/orders?page=42";
    let token = "same-benchmark-token";
    let cookie = "session=same-session-value";

    let axum_request = http::Request::builder()
        .method("GET")
        .uri(uri)
        .header("satoken", token)
        .header("cookie", cookie)
        .body(())
        .unwrap();
    let actix_request = actix_web::test::TestRequest::get()
        .uri(uri)
        .insert_header(("satoken", token))
        .insert_header(("cookie", cookie))
        .to_http_request();
    let poem_request = poem::Request::builder()
        .method("GET".parse().unwrap())
        .uri(uri.parse().unwrap())
        .header("satoken", token)
        .header("cookie", cookie)
        .finish();
    let tonic_request = http::Request::builder()
        .method("GET")
        .uri(uri)
        .header("satoken", token)
        .header("cookie", cookie)
        .body(())
        .unwrap();
    let tonic_request = TonicCapturedRequest::from_http(&tonic_request);

    let mut group = criterion.benchmark_group("same_contract_request_adapter");
    group.throughput(Throughput::Elements(1));
    group.bench_function("axum_0_8", |bencher| {
        let adapter = AxumRequestAdapter::new(&axum_request);
        bencher.iter(|| std::hint::black_box(exercise(&adapter)));
    });
    group.bench_function("actix_web_4", |bencher| {
        let adapter = ActixRequestAdapter::new(&actix_request);
        bencher.iter(|| std::hint::black_box(exercise(&adapter)));
    });
    group.bench_function("poem_3", |bencher| {
        let adapter = PoemRequestAdapter::new(&poem_request);
        bencher.iter(|| std::hint::black_box(exercise(&adapter)));
    });
    group.bench_function("tonic_0_12_snapshot", |bencher| {
        bencher.iter(|| std::hint::black_box(exercise(&tonic_request)));
    });
    group.finish();
}

criterion_group!(benches, framework_adapter_comparison);
criterion_main!(benches);
