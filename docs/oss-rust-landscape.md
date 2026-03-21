# Rust OSS Landscape For Ferrovia

Checked on 2026-03-21 against current crates.io/docs.rs metadata and upstream repositories.

## Goal

This document lists the maintained Rust OSS components that are plausible building blocks for ferrovia. The point is not to maximize dependency count. The point is to avoid rewriting mature subdomain logic while keeping strict `svgo@4.0.1` parity for `preset-default`.

## Recommended Stack

### Adopt Or Strongly Consider

- `xmlparser`
- `xmlparser` `0.13.6` `MIT/Apache-2.0`
  - Role: low-level XML tokenizer
  - Fit: good candidate if ferrovia wants to replace parts of the handwritten parser without adopting a foreign tree model
  - Why: zero-allocation tokenizer and small scope
  - Risk: still leaves tree construction, source-detail preservation, and serializer contracts to ferrovia

- `svgtypes` `0.16.1` `Apache-2.0 OR MIT`
  - Role: SVG value-domain parsing
  - Fit: best current candidate for path, transform, paint, length, viewBox, and related subparsers
  - Why: narrow, maintained, and directly aligned with several pending geometry plugins
  - Risk: parser output still needs ferrovia-owned parity logic and serialization

- `cssparser` `0.37.0` `MPL-2.0`
  - Role: CSS syntax parsing
  - Fit: good building block for `mergeStyles`, `inlineStyles`, and `minifyStyles`
  - Why: maintained Servo component with well-defined scope
  - Risk: low-level API and MPL-2.0 licensing require deliberate adapter boundaries

- `simplecss` `0.2.2` `Apache-2.0 OR MIT`
  - Role: small CSS parser plus selector support
  - Fit: attractive for a minimal stylesheet subset and worth evaluating before integrating `selectors`
  - Why: smaller scope than full Servo-style selector/cascade stacks and already used in the linebender ecosystem
  - Risk: feature coverage is intentionally limited

- `selectors` `0.36.1` `MPL-2.0`
  - Role: CSS selector matching
  - Fit: viable only if ferrovia needs a more complete selector engine and can bridge it to the internal tree cheaply
  - Why: mature implementation from the Servo ecosystem
  - Risk: integration complexity is materially higher than `simplecss`, and it assumes a richer element model

- `euclid` `0.22.14` `MIT OR Apache-2.0`
  - Role: geometry primitives and transforms
  - Fit: useful utility layer for matrix math and transform normalization
  - Why: mature, focused, and broadly reused
  - Risk: not SVG-specific; parity semantics stay in ferrovia

- `kurbo` `0.13.0` `Apache-2.0 OR MIT`
  - Role: 2D geometry and curve operations
  - Fit: good candidate for later path and curve manipulation if the geometry wave needs robust curve tooling
  - Why: maintained linebender geometry library
  - Risk: may be more power than the early plugin waves need

### Reference Or Optional Backend Use

- `svgparser` `0.8.1` `MIT/Apache-2.0`
  - Role: pull-based SVG parser
  - Fit: useful reference implementation and potential tokenizer alternative, but less obviously aligned to ferrovia's current tree and serializer contracts than `xmlparser`

- `lyon_path` `1.0.19` `MIT OR Apache-2.0`
  - Role: path storage and iteration
  - Fit: good optional backend helper once path rewriting becomes heavier
  - Risk: stronger fit for geometry pipelines than for byte-parity plugin logic

- `tiny-skia-path` `0.12.0` `BSD-3-Clause`
  - Role: path representation from tiny-skia
  - Fit: potentially useful for backend path operations, but lower priority than `svgtypes` plus lighter geometry helpers

- `usvg` `0.47.0` `Apache-2.0 OR MIT`
  - Role: strongly normalized SVG IR
  - Fit: excellent reference and backend inspiration for geometry and normalization behavior
  - Risk: too normalizing to serve as the frontend source of truth for ferrovia

### Usually Not Worth Adopting As Core Building Blocks

- `quick-xml` `0.39.2` `MIT`
  - Fast and maintained, but ferrovia gains less from it than from a tokenizer-level crate because strict output parity depends on ferrovia's own tree and serializer model.

- `roxmltree` `0.21.1` `MIT OR Apache-2.0`
  - Excellent read-only XML tree for analysis and tools, but it does not fit ferrovia's mutable arena model.

- `xot` `0.31.2` `MIT`
  - Capable XML tree/manipulation library, but it would replace rather than support ferrovia's internal document model.

- `sxd-xpath` `0.4.2` `MIT/Apache-2.0`
  - Useful if XPath is a feature target; it is not a good driver for `preset-default` parity work.

- HTML-oriented stacks such as `scraper`, `html5ever`, or `kuchikiki`
  - Strong libraries, but the model mismatch is too large for XML/SVG-centric optimization.

## Category Notes

### XML And SVG Parsing

Best fit for ferrovia is still:

1. ferrovia-owned document model and serializer
2. handwritten parser or `xmlparser`-backed tokenizer
3. targeted SVG subparsers via `svgtypes`

That preserves control over:

- comments
- processing instructions
- XML declaration details
- doctype handling
- quote style
- node ordering
- mutation semantics

### Query And Selector Engine

There is no obvious drop-in query engine that is both:

- source-faithful for SVG/XML
- compatible with ferrovia's arena tree
- broad enough to cover CSS selector needs

The practical route is:

1. keep internal tree traversal and simple structural queries custom
2. evaluate `simplecss` first for narrow stylesheet and selector needs
3. escalate to `selectors` only if the style wave proves that the lighter option is too weak
4. do not adopt XPath as an architectural dependency

### CSS And Style Handling

Recommended order:

1. continue using ferrovia-owned inline style helpers for trivial cases
2. add `cssparser` or `simplecss` behind a private adapter for stylesheet parsing and selective minification
3. avoid a full browser-grade cascade engine

This keeps the CSS wave parity-oriented instead of turning it into a browser project.

### Geometry And Path Work

Recommended order:

1. `svgtypes` as the first real dependency
2. `euclid` for transform math if needed
3. `kurbo` or `lyon_path` only when concrete plugins need richer path operations
4. use `usvg` as a reference implementation, not as the document frontend

## Short Decision Summary

- Keep custom:
  - document tree
  - serializer
  - core query traversal
  - output-sensitive mutation logic

- Strongest adoption candidates:
  - `xmlparser`
  - `svgtypes`
  - `cssparser`
  - `simplecss`
  - optionally `euclid`

- Keep at arm's length:
  - `selectors`
  - `kurbo`
  - `lyon_path`
  - `usvg`

- Avoid as core architecture:
  - `quick-xml`
  - `roxmltree`
  - `xot`
  - `sxd-xpath`
  - HTML DOM/query stacks
