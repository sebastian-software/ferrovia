# ADR 0013: Direct-Port Rewrite Reset

Date: 2026-03-24
Status: Accepted

## Context

The previous ferrovia core accumulated a Rust-first implementation that partially matched SVGO but no longer served as the chosen foundation for a strict file-for-file rewrite.

The project now intentionally pivots to a hard reset of the core while preserving external interfaces and verification harnesses.

## Decision

- Keep CLI, N-API, Node wrapper, repo metadata, and verification scripts.
- Reset `ferrovia-core` to a new SVGO-shaped module tree.
- Port upstream files in topological order with minimal reinterpretation.
- Build parity-critical external dependencies as separate workspace compat crates.
- Treat all prior core implementation ADRs as historical context rather than the active implementation path.

## Consequences

- The codebase becomes smaller and more explainable in the short term, but temporarily supports fewer plugins.
- The rewrite now optimizes for semantic traceability to upstream rather than local elegance.
- Compatibility crates are first-class workspace members and part of the rewrite surface.
