#!/usr/bin/env bash
set -euo pipefail

scope="${1:-local}"
criterion_args=(--sample-size 10 --warm-up-time 0.1 --measurement-time 0.2 --noplot)

case "${scope}" in
    local)
        cargo bench -p sa-token-core --bench core_hot_paths -- "${criterion_args[@]}"
        cargo bench -p sa-token-core --bench auth_throughput -- "${criterion_args[@]}"
        cargo bench -p sa-token-performance-tests --bench framework_adapter_comparison -- "${criterion_args[@]}"
        cargo run -p sa-token-performance-tests --release --bin dhat_auth_memory
        cargo run -p sa-token-performance-tests --release --bin jemalloc_auth_memory
        ;;
    real-backends)
        : "${SA_TOKEN_BENCH_REDIS_URL:?set SA_TOKEN_BENCH_REDIS_URL to a real Redis service}"
        : "${SA_TOKEN_BENCH_DATABASE_URL:?set SA_TOKEN_BENCH_DATABASE_URL to a real PostgreSQL service}"
        cargo bench -p sa-token-performance-tests --bench backend_latency -- "${criterion_args[@]}"
        ;;
    linux-regression)
        command -v valgrind >/dev/null || {
            echo "valgrind is required for linux-regression" >&2
            exit 1
        }
        command -v gungraun-runner >/dev/null || {
            echo "gungraun-runner is required; install version 0.19.4" >&2
            exit 1
        }
        cargo bench -p sa-token-core --bench instruction_hot_paths
        ;;
    *)
        echo "usage: $0 [local|real-backends|linux-regression]" >&2
        exit 2
        ;;
esac
