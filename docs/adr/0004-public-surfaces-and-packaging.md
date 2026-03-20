# ADR 0004: Public Surfaces And Packaging

- Status: accepted
- Date: 2026-03-20

## Context

The project needs to serve both Rust-native consumers and existing Node-based SVGO usage. Those surfaces must share one optimization core so behavior does not fork by frontend.

## Decision

- The Rust core lives in `crates/ferrovia-core`.
- The native CLI lives in `crates/ferrovia-cli`.
- The Node binding lives in `crates/ferrovia-napi`.
- The JS package wrapper lives in `packages/node`.
- The Node package resolves config and loads the compiled `.node` addon, while the Rust core stays free of JS execution concerns.

## Consequences

- CLI, Rust API, and Node package all converge on the same optimizer implementation.
- Packaging issues can be solved at the boundary without polluting core optimization code.
- `svgo.config.mjs` support belongs in the Node-facing layer, not in the native Rust CLI.

