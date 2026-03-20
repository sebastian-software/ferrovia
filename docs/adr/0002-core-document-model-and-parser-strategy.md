# ADR 0002: Core Document Model And Parser Strategy

- Status: accepted
- Date: 2026-03-20

## Context

SVGO plugins need whole-document visibility often enough that a pure streaming rewrite architecture would make parity difficult. At the same time, a heavy DOM-like structure would give away too much of Rust's memory and CPU advantage.

## Decision

- ferrovia uses an arena-backed document model with stable node ids.
- Parsing is byte-oriented and optimized for direct scanning, with `memchr`-style fast paths where possible.
- The serializer is deterministic and preserves enough source detail to support byte-oriented parity for supported features.
- The design explicitly prefers a compact tree plus targeted lazy parsing over a general-purpose XML DOM.

## Consequences

- Global and structural plugins can be implemented against a stable internal model.
- The core remains small enough to optimize aggressively without carrying browser-style DOM baggage.
- Some advanced SVGO areas such as CSS, transforms, and path data will be added as focused subparsers rather than forcing a single eager parse of everything.

