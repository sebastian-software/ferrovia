#!/usr/bin/env bash
set -euo pipefail

CORPUS_DIR="${1:-${FERROVIA_SVGO_TEST_SUITE_DIR:-vendor/svgo-test-suite}}"
PROFILE="${2:-${FERROVIA_SVGO_TEST_SUITE_PROFILE:-full-corpus}}"
export FERROVIA_SVGO_TEST_SUITE_DIR="$CORPUS_DIR"

case "$PROFILE" in
  smoke-20)
    export FERROVIA_SVGO_TEST_SUITE_LIMIT=20
    ;;
  sample-100)
    export FERROVIA_SVGO_TEST_SUITE_LIMIT=100
    ;;
  milestone-500)
    export FERROVIA_SVGO_TEST_SUITE_LIMIT=500
    ;;
  full-corpus)
    unset FERROVIA_SVGO_TEST_SUITE_LIMIT
    ;;
  *)
    if [[ "$PROFILE" =~ ^[0-9]+$ ]]; then
      export FERROVIA_SVGO_TEST_SUITE_LIMIT="$PROFILE"
    else
      echo "unknown corpus profile: $PROFILE" >&2
      exit 1
    fi
    ;;
esac

cargo test -p ferrovia-core --test corpus -- --nocapture
