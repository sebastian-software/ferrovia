# ADR 0012: Semantic Direct-Port Spike

Date: 2026-03-24
Status: Accepted

## Context

`ferrovia` has reached a measured baseline where `preset-default` is feature-complete, but the remaining parity wall against `svgo@4.0.1` is narrowing more slowly than desired.

The current parity strategy is:

- preserve the current Rust-first document and serializer model
- close oracle and corpus mismatches incrementally
- prefer idiomatic local abstractions and optimize cluster-by-cluster

That strategy produced real progress, but the remaining mismatches are concentrated in domains where SVGO behavior is defined by subtle helper ordering, deoptimizations, tie-breaks, and traversal details:

- `cleanupIds`
- `convertPathData`
- `inlineStyles`

The project therefore needs a strategy-validation spike that answers whether a more literal semantic transliteration of upstream SVGO logic closes the remaining gap faster than continued local reinterpretation.

## Decision

`ferrovia` will run a dedicated semantic direct-port spike on a parallel branch before committing to a broader rewrite.

The spike changes implementation strategy, not product surface:

- public Rust, CLI, and Node APIs remain unchanged
- the current parser, arena AST, and serializer remain the source of truth during the spike
- the existing parity path is frozen as the measured baseline

Inside the spike, semantic fidelity takes precedence over Rust idiomatics:

- upstream helper boundaries should remain recognizable
- plugin orchestration should stay close to SVGO structure
- deopts, pass ordering, and tie-breaks should be preserved even when the Rust is temporarily awkward

The spike is intentionally scoped to three representative plugin families:

1. `cleanupIds`
2. `convertPathData`
3. `inlineStyles`

These three cover the hardest remaining parity mechanisms:

- ID and reference rewriting
- path canonicalization and transform interaction
- selector, stylesheet, and CSS cleanup behavior

## Consequences

### Positive

- The project gets a decision-grade answer instead of continuing to guess whether the current approach is still the fastest route to parity.
- The spike keeps the stable baseline intact and measurable.
- A thin internal compatibility layer can later be reused for additional direct ports if the spike succeeds.

### Negative

- The codebase now temporarily contains two implementation styles for overlapping behavior.
- Some spike code will be intentionally less idiomatic than the rest of `ferrovia`.
- The spike may prove to be a no-go, in which case part of the work becomes design evidence rather than production code.

### Neutral constraints

- MIT/Apache-licensed dependencies remain preferred.
- MPL dependencies are not adopted by default and, if ever introduced later, must stay behind private adapters.
- The spike does not justify replacing the current DOM, parser, or serializer with third-party frameworks.

## Validation

The spike is only a go if it demonstrates leverage against the current stable baseline:

- `sample-100` drops below the current stable `18 / 100`, or
- the hotspot acceptance corpus is reduced by at least `50%`, or
- the remaining diffs become clearly narrower and attributable to a smaller helper surface

If the spike does not achieve one of those outcomes, the result is a no-go and the main strategy remains the corpus-driven parity path.
