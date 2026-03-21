# ADR 0009: Minimal Semantic Collections

## Status

Accepted

## Context

Phase 2 of ferrovia is about the semantic/default/hidden plugin block:

- `cleanupEnableBackground`
- `removeNonInheritableGroupAttrs`
- `removeUnknownsAndDefaults`
- `removeUselessStrokeAndFill`
- `removeHiddenElems`

These plugins do not need a full SVG semantics engine, but they do need a small, shared model of SVG collections and defaults. Without that model, each plugin tends to reimplement its own view of presentation attributes, inheritable attributes, container behavior, and default values. That would make parity fragile and increase the risk of inconsistent rules across plugins.

## Decision

Ferrovia will maintain a minimal internal semantic collection layer that covers only the data required by the current Phase 2 plugins:

- `presentation` attributes, split into inheritable and non-inheritable group attributes where needed.
- `default` attribute values for the small subset of elements and attributes exercised by the current plugin wave.
- `container` and `non-rendering` element groups needed for empty/hidden element removal and group cleanup.
- `shape`, `text`, `animation`, and `reference` element/attribute groups needed to decide whether a node can be collapsed, hidden, or preserved.
- `styled reference` attributes and a minimal filter/visibility view sufficient for subtree deopt decisions.

This layer must stay implementation-oriented:

- It is allowed to be incomplete outside the current plugin wave.
- It should grow only when a concrete plugin needs a new rule or collection member.
- It must remain small enough that the expected behavior can be verified against SVGO fixtures and oracle tests.
- It must not become a generic SVG schema, parser, or CSS engine.

The data model should be expressed as a small set of internal lookup helpers and tables rather than a public API. Plugins may consume the helpers directly, but the helpers remain private to the Rust core.

## Consequences

- Phase 2 plugins can share the same semantic view of SVG without each one carrying its own ad hoc constants.
- Parity bugs caused by mismatched definitions of “inheritable”, “hidden”, or “container” are less likely.
- The codebase can add new collection entries incrementally as Phase 2 expands, without committing to a full SVG taxonomy up front.
- Later phases can introduce their own narrow models for CSS and geometry without overloading this semantic layer.
