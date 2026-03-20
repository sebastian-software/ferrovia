#!/usr/bin/env bash
set -euo pipefail

for config in tests/fixtures/oracle/*.config.json; do
  svg="${config%.config.json}.svg"
  echo "== $(basename "${svg}") =="
  bash ./scripts/bench-fixture.sh "$svg" "$config"
done

