# ADR 0010: Dependency Leverage And OSS Adoption

## Status

Accepted

## Context

ferrovia targets byte-oriented parity with `svgo@4.0.1` for `preset-default`. That makes some parts of the system unusually sensitive to normalization and output drift:

- the XML/SVG frontend parser
- the serializer
- tree mutation and node ordering
- plugin behaviors that depend on exact source structure

At the same time, reimplementing every supporting concern from scratch would waste time and increase maintenance risk. The project therefore needs a stable rule for where custom code is required and where maintained Rust crates should be adopted.

The main pressure points are:

- XML/SVG tokenization and parsing
- query and selector matching
- CSS and inline-style handling
- geometry, path, transform, color, and numeric parsing

## Decision

ferrovia will adopt a hybrid dependency strategy:

- Keep source-faithful, byte- and output-critical layers custom by default.
- Prefer maintained third-party crates behind narrow internal adapters for subdomains that are not themselves the canonical output model.
- Reject dependencies that require ferrovia to adopt a foreign DOM or a fully normalizing intermediate representation as the source of truth for optimization.

This decision resolves into the following rules.

### 1. Parser And Serializer

The frontend parser and serializer remain ferrovia-owned.

- The current arena-backed document model remains the source of truth.
- The parser may selectively adopt a lower-level tokenizer such as `xmlparser` if it reduces parser complexity without forcing an alien tree model or output normalization.
- `quick-xml`, `roxmltree`, `xot`, and `usvg` are not adopted as the frontend source of truth because they either impose their own tree model or normalize too aggressively for strict SVGO parity.

### 2. Query And Selector Strategy

ferrovia will not adopt a foreign DOM just to gain querying.

- Querying over the internal tree remains a ferrovia responsibility.
- Selector parsing or matching crates may be used only if they can be adapted to the internal node model through a thin integration layer.
- XPath-style libraries are out of scope for `preset-default` parity and should not drive core architecture.

The preferred direction is a minimal internal query layer plus optional selector support for the concrete plugin waves that require it.

### 3. CSS And Style Handling

ferrovia should leverage maintained CSS building blocks, but not adopt a general browser-grade CSS engine.

- For inline declarations and stylesheet parsing, narrow crates such as `cssparser` or `simplecss` are acceptable behind internal adapters.
- `selectors` is acceptable only if its integration cost against the custom tree stays lower than maintaining equivalent logic ourselves for the needed subset.
- The project explicitly avoids adopting a full browser-style cascade engine for `preset-default`.

### 4. Geometry, Path, Transform, Color, And Numeric Handling

ferrovia should prefer maintained subparsers and geometry helpers here.

- `svgtypes` is the preferred dependency for parsing SVG value domains such as path data, transforms, paint, lengths, and view boxes.
- Additional math or path crates such as `lyon_path`, `kurbo`, or `euclid` are acceptable when a plugin needs robust internal operations on geometry.
- `usvg` may be used as a reference model or as inspiration for narrowly scoped conversions, but not as the optimization frontend or canonical document representation.

### 5. Integration Discipline

Every adopted dependency must satisfy all of the following:

- actively maintained enough to be a reasonable long-term bet
- compatible license for ferrovia's OSS distribution
- narrower scope than the ferrovia layer it supports
- hidden behind an internal adapter, not leaked into the public API
- covered by differential tests proving that the dependency does not change SVGO parity

## Consequences

- ferrovia avoids writing low-level parsers for every SVG subdomain while preserving control over source-sensitive behavior.
- Query and selector support stays intentionally small and internal instead of dragging in a second tree model.
- CSS and geometry work can move faster by reusing maintained crates, but the project keeps final control over semantics and serialization.
- Some crates that look attractive in isolation, especially `usvg`, remain deliberately limited to backend or reference roles because they normalize too early for the parity target.
