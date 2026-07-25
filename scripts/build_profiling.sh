#!/usr/bin/env bash
set -euo pipefail

export RUSTFLAGS="${RUSTFLAGS:-} -C force-frame-pointers=yes"
exec cargo build --profile profiling "$@"
