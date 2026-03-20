#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 2 ]]; then
  echo "usage: $0 <svg-path> <config-json-path>" >&2
  exit 1
fi

svg_path="$1"
config_path="$2"

echo "ferrovia-cli"
/usr/bin/time -l cargo run --release -p ferrovia-cli -- "$svg_path" "$config_path" >/dev/null

echo "svgo oracle"
/usr/bin/time -l node ./scripts/run-svgo-oracle.mjs "$svg_path" "$config_path" >/dev/null

