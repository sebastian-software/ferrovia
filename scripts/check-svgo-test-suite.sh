#!/usr/bin/env bash
set -euo pipefail

CORPUS_DIR="${1:-${FERROVIA_SVGO_TEST_SUITE_DIR:-vendor/svgo-test-suite}}"
export FERROVIA_SVGO_TEST_SUITE_DIR="$CORPUS_DIR"

cargo test -p ferrovia-core --test corpus -- --nocapture
