# ADR 0011: Minimal CSS Stylesheet Model For Style Wave

## Status

Accepted

## Context

Phase 3 of ferrovia covers the style- and defs-related plugin wave:

- `mergeStyles`
- `inlineStyles`
- `minifyStyles`
- `cleanupIds`

These plugins need a shared but intentionally small internal CSS model. The
goal is not a generic browser-grade CSS engine. The goal is to support the
specific selector matching, declaration parsing, and stylesheet rewriting that
SVGO v4.0.1 actually uses in this wave.

## Decision

Ferrovia will use a minimal internal stylesheet layer built around narrow,
private helpers:

- declaration parsing and serialization for inline `style=""` content
- stylesheet parsing into rules with selector text, specificity, and
  declarations
- selector matching against the ferrovia tree for the subset supported by the
  chosen parser stack
- simple stylesheet rewrites that operate on `<style>` text and preserve SVGO's
  observable merge/minify behavior

This layer remains implementation-oriented:

- `mergeStyles` may operate as a document-global text rewrite, matching SVGO's
  current behavior rather than enforcing stricter structural checks
- `inlineStyles` and `cleanupIds` may extend the layer only when a concrete
  parity case requires it
- unsupported CSS constructs should degrade conservatively rather than inventing
  partial semantics

The current dependency strategy remains:

- `simplecss` for selector parsing and matching
- ferrovia-owned tree traversal and style rewrite logic

No new public API is introduced by this layer.

## Consequences

- Phase 3 can share one internal notion of parsed stylesheet rules and selector
  matching.
- `mergeStyles` can be implemented first as a low-risk rewrite without forcing
  the full complexity of `inlineStyles`.
- The CSS model stays deliberately smaller than a browser engine, which reduces
  both maintenance surface and parity drift risk.
- Later style plugins can build on the same parsed rule representation instead
  of re-parsing ad hoc CSS strings in multiple places.
