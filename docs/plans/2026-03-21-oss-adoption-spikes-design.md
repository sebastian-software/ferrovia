# OSS Adoption Spikes Design

- Date: 2026-03-21
- Status: approved

## Goal

Reduce unnecessary custom implementation work in ferrovia without compromising strict `svgo@4.0.1` `preset-default` parity.

## Scope

Three narrow spikes:

1. add `svgtypes` behind a ferrovia-owned geometry adapter
2. add `simplecss` behind a ferrovia-owned stylesheet and selector adapter
3. add an `xmlparser` tokenizer spike to evaluate frontend replacement potential without changing the production parser

## Chosen Approach

Keep the current parser, serializer, tree, and mutation logic as the production source of truth.

Adopt maintained crates only behind internal adapters:

- `svgtypes` becomes the first real production dependency for SVG value-domain parsing
- `simplecss` becomes a lightweight stylesheet and selector building block
- `xmlparser` is introduced only as an evaluation path and testable tokenizer abstraction

## Why This Approach

- It creates immediate leverage where the crates are strongest.
- It avoids premature migration of the most parity-sensitive layers.
- It gives ferrovia reusable internal building blocks for the upcoming style and geometry waves.

## Deliverables

- internal geometry module for path and transform parsing
- internal stylesheet module for inline declarations, stylesheet rules, and selector matching on the ferrovia tree
- internal xml tokenizer probe that captures xmlparser coverage for the syntax classes ferrovia cares about
- tests proving these adapters work on representative SVG inputs

## Non-Goals

- no parser frontend switch
- no public API expansion
- no full CSS engine
- no geometry plugin rewrites yet
