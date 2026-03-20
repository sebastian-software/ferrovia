# ADR 0003: Verification And Regression Oracle

- Status: accepted
- Date: 2026-03-20

## Context

A compatibility port is only credible if parity is measured against the real upstream implementation. Recreating tests by hand would risk slowly encoding ferrovia's own behavior as the new truth.

## Decision

- Node-based `svgo@4.0.1` is used as the reference oracle in development and CI.
- The primary gate is bytewise comparison between upstream SVGO output and ferrovia output on shared fixtures.
- Upstream-style fixtures are preferred over hand-written bespoke tests whenever practical.
- A regression runner must cover:
  - Rust tests
  - SVGO oracle execution
  - N-API build
  - Node smoke validation

## Consequences

- ferrovia keeps a temporary dependency on the JS toolchain for verification, not for product runtime semantics in the Rust core.
- New plugin waves should not be considered complete until they have differential tests.
- Performance work must not bypass parity checks.

