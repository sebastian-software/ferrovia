# ferrovia

`ferrovia` is a SVGO-compatible SVG optimizer written in Rust.

The current focus is byte-level behavioral parity with `svgo@4.0.1`, verified through differential tests and vendored fixture coverage. The project contains:

- `ferrovia-core`: the Rust optimizer library
- `ferrovia-cli`: the native CLI
- `ferrovia-napi` and `packages/node`: the Node.js bindings

## Status

This project is under active development.

- Differential verification against `svgo@4.0.1` is part of the normal workflow.
- The port is intentionally incremental: plugins are implemented in small verified waves.
- The implementation is not yet feature-complete relative to upstream SVGO.

## Goals

- Preserve SVGO-compatible config and plugin semantics where practical.
- Keep the Rust core CPU- and memory-conscious.
- Verify behavior against upstream fixtures and a pinned Node-based oracle.

## Workspace

```text
crates/ferrovia-core
crates/ferrovia-cli
crates/ferrovia-napi
packages/node
docs/adr
tests
```

## Development

Prerequisites:

- Rust toolchain
- Node.js
- pnpm

Install dependencies:

```bash
pnpm install
```

Run the main verification steps:

```bash
cargo test
cargo clippy --all-targets --all-features -- -D warnings
bash ./scripts/check-regressions.sh
```

Build the Node addon:

```bash
pnpm build:napi
```

## Project Documents

- Architecture decisions live in `docs/adr/`.
- Ongoing compatibility and regression checks live under `tests/` and `scripts/`.

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md).

## Security

See [SECURITY.md](./SECURITY.md).

## License

MIT. See [LICENSE](./LICENSE).
