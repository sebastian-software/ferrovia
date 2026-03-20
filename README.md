# ferrovia

`ferrovia` is a Rust-first SVG optimizer targeting SVGO-compatible behavior with a shared core for Rust, CLI, and Node consumers.

## Workspace

- `crates/ferrovia-core`: parser, AST, serializer, config model, plugin pipeline
- `crates/ferrovia-cli`: native CLI
- `crates/ferrovia-napi`: Node/N-API bindings
- `packages/node`: JS wrapper and config loader
- `scripts/run-svgo-oracle.mjs`: pinned SVGO v4.0.1 reference harness

