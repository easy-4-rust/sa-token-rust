# Observability and profiling

Sa-Token keeps diagnostics opt-in and application-owned. The core emits a
small, privacy-safe vocabulary; the application chooses exporters, endpoints,
retention and access control.

## Default telemetry contract

- Spans: `sa_token.auth`, `sa_token.plugin.lifecycle`, `sa_token.storage`.
- Metrics: `sa_token_auth_total`, `sa_token_auth_duration_seconds`,
  `sa_token_plugin_lifecycle_total`, `sa_token_storage_duration_seconds`,
  `sa_token_event_total`.
- Allowed labels are bounded outcomes, operation names, plugin names and
  configured backend names.
- Token values, login IDs, storage keys and values, paths, device IDs, nonces,
  arbitrary errors and request bodies must never be span fields or metric
  labels.

Enable core metrics through `sa-token-core/metrics`, or use
`sa-token-plugin-observability` with its default features. Wrap storage before
constructing the manager:

```rust
let storage = Arc::new(ObservedStorage::new("redis", redis_storage));
let manager = SaTokenManager::new(storage, config);
```

The observability plugin publishes runtime-local state and an optional reload
control. It deliberately does not call `set_global_default`. Compose the
reloadable filter, formatting/JSON layer and OpenTelemetry layer in the
application binary.

## Controlled production diagnostics

`sa-token-diagnostics` has no default features:

- `tokio-console`: returns a builder for a composable console layer. Build with
  `RUSTFLAGS="--cfg tokio_unstable"` and never expose its port publicly.
- `pprof`: `PprofController` permits one CPU capture at a time and caps capture
  duration. An application endpoint must add administrator authentication,
  rate limiting and audit logging.
- `dhat-heap`: re-exports DHAT for development binaries. The final application,
  not a library, must choose the global allocator and profiler guard.

For readable native profiles:

```bash
./scripts/build_profiling.sh -p your-application
```

The `profiling` Cargo profile preserves full debug symbols and the script forces
frame pointers. Use Criterion locally to compare distributions. Gungraun runs
instruction-level regression checks under Valgrind on Linux CI, avoiding noisy
wall-clock thresholds.

For a local Linux run, install both Valgrind and the runner version matching
the workspace dependency, then use the checked wrapper:

```bash
cargo install gungraun-runner --version 0.19.4 --locked
./scripts/run_performance_suite.sh linux-regression
```

GitHub Actions uses `gungraun/setup-gungraun@v1` to install a matching runner
and Valgrind together.

## Reproducible performance gates

Run the local, dependency-free suite:

```bash
./scripts/run_performance_suite.sh local
```

It executes core hot paths, the complete login/auth/logout lifecycle, the same
request contract through Axum 0.8, Actix Web 4, Poem 3 and Tonic 0.12 adapters,
plus DHAT and jemalloc memory workloads.

The real-backend suite never substitutes memory storage or silently skips:

```bash
export SA_TOKEN_BENCH_REDIS_URL=redis://127.0.0.1:6379/0
export SA_TOKEN_BENCH_DATABASE_URL=postgres://postgres:password@127.0.0.1/satoken
./scripts/run_performance_suite.sh real-backends
```

The backend benchmark measures storage GET/SET, valid authentication and
login/logout over actual network connections. CI provisions isolated Redis and
PostgreSQL services.

Memory defaults are calibrated for 500 DHAT cycles and 10,000 jemalloc cycles.
They can be tightened or adapted to a dedicated runner through:

- `SA_TOKEN_DHAT_MAX_TOTAL_BYTES`, `SA_TOKEN_DHAT_MAX_PEAK_BYTES`,
  `SA_TOKEN_DHAT_MAX_LIVE_BYTES`
- `SA_TOKEN_JEMALLOC_MAX_ALLOCATED_GROWTH`,
  `SA_TOKEN_JEMALLOC_MAX_RESIDENT_GROWTH`

Criterion wall-clock numbers are evidence and trend data, not hard CI limits:
shared runners are noisy. Callgrind instruction counts, DHAT allocations and
explicit jemalloc growth limits are the deterministic failure gates.

## Incident workflow

1. Raise only the relevant module filter through the reload control.
2. Locate the slow operation with `sa_token.auth` and `sa_token.storage` spans.
3. Use Tokio console for task starvation or wakeup problems.
4. Request a short authenticated pprof capture for CPU hot spots.
5. Reproduce locally with Criterion; use DHAT for allocation-heavy paths.
6. Use eBPF, `perf`, `lldb` or `gdb` only when pre-instrumented evidence is
   insufficient.
