#!/usr/bin/env bash
set -euo pipefail

scope="${1:-fast}"

if [[ "${scope}" != "fast" && "${scope}" != "full" ]]; then
    echo "usage: $0 [fast|full]" >&2
    exit 2
fi

check_features() {
    local package="$1"
    local features="$2"
    cargo check -p "${package}" --no-default-features --features "${features}"
}

# Baseline and the four runtime-scoped security plugins.
cargo check --workspace
cargo check -p sa-token-rust --features security-plugins
cargo check -p sa-token-rust --features plugin-observability
cargo test -p sa-token-integration-tests --test java_golden_contract
cargo test \
    -p sa-token-plugin-apikey \
    -p sa-token-plugin-jwt \
    -p sa-token-plugin-oauth2 \
    -p sa-token-plugin-sso
cargo test -p sa-token-plugin-observability --all-features
cargo check -p sa-token-diagnostics --features pprof,tokio-console,dhat-heap

# Every production framework binding with its default in-memory backend.
check_features sa-token-plugin-axum "axum-08,memory"
check_features sa-token-plugin-actix-web "v4,memory"
check_features sa-token-plugin-rocket "v05,memory"
check_features sa-token-plugin-warp "warp-03,memory"
check_features sa-token-plugin-poem "poem-03,memory"
check_features sa-token-plugin-salvo "v079,memory"
check_features sa-token-plugin-tide "tide-017,memory"
check_features sa-token-plugin-gotham "v074,memory"
check_features sa-token-plugin-ntex "v212,memory"
check_features sa-token-plugin-tonic "tonic-012,memory"

if [[ "${scope}" == "full" ]]; then
    # Storage selection must compile independently; all-features is not a valid
    # substitute because framework major-version features can be exclusive.
    for backend in redis database; do
        check_features sa-token-plugin-axum "axum-08,${backend}"
        check_features sa-token-plugin-actix-web "v4,${backend}"
        check_features sa-token-plugin-rocket "v05,${backend}"
        check_features sa-token-plugin-warp "warp-03,${backend}"
        check_features sa-token-plugin-poem "poem-03,${backend}"
        check_features sa-token-plugin-salvo "v079,${backend}"
        check_features sa-token-plugin-tide "tide-017,${backend}"
        check_features sa-token-plugin-gotham "v074,${backend}"
        check_features sa-token-plugin-ntex "v212,${backend}"
        check_features sa-token-plugin-tonic "tonic-012,${backend}"
    done
fi
