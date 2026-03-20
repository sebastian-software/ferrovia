# ADR 0001: Product Scope And Compatibility Target

- Status: accepted
- Date: 2026-03-20

## Context

ferrovia is intended as a Rust-first successor path for SVGO-style optimization with a focus on performance, memory efficiency, and migration from existing JS workflows. The project needs a single compatibility anchor to avoid drifting behavior during the port.

## Decision

- ferrovia v1 targets `SVGO v4.0.1` as the reference semantics baseline.
- The compatibility goal is byte-identical output for the same input, config, and plugin order whenever the referenced plugin behavior is implemented.
- v1 is not a brand new optimizer with loosely similar behavior; it is a compatibility-oriented port.
- The shipped product surface in v1 includes:
  - a Rust core library
  - a native CLI
  - an official Node/N-API entry point for migration from JS-based workflows

## Consequences

- Upstream behavior changes must be treated as explicit migration decisions, not silent drift.
- Output formatting is part of the compatibility contract, not an implementation detail.
- Work is prioritized by parity and verifiability before broader feature invention.

