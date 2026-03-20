#!/usr/bin/env bash
set -euo pipefail

cargo test
pnpm test:oracle
pnpm build:napi
pnpm test:node

