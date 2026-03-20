# ADR 0005: Config And Plugin Model

- Status: accepted
- Date: 2026-03-20

## Context

Config migration is one of the biggest adoption costs for a compatibility port. A Rust-native config shape would be cleaner internally but would immediately weaken drop-in migration.

## Decision

- ferrovia uses an SVGO-shaped configuration model for the public surface.
- Plugin ordering is explicit and behaviorally important.
- `preset-default` is represented as a first-class pipeline expansion step with override support.
- v1 supports built-in plugins plus Rust-native extension points.
- General JS custom plugins are out of scope for v1 compatibility.

## Consequences

- The config model stays familiar to existing SVGO users.
- Built-in plugin waves can be ported incrementally while preserving the long-term config contract.
- Custom extension remains possible in Rust without embedding a general JS plugin runtime in the core.

