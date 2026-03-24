# SVGO Direct-Port Spike Report

Date: 2026-03-24
Reference: `svgo@4.0.1`
Status: Initial spike result

## Scope

This report covers the first decision-grade spike run for the semantic direct-port path.

Current spike characteristics:

- baseline parity path frozen as control
- spike entrypoint and comparison harness added
- `cleanupIds` ported in an upstream-structured style on the current tree
- `inlineStyles` moved behind the spike compatibility layer
- `convertPathData` routed through the spike entrypoint while still using the current path backend

## Acceptance Corpus Result

Hotspot corpus:

- `animate-elem-60-t.svg`
- `animate-elem-62-t.svg`
- `animate-elem-78-t.svg`
- `animate-elem-83-t.svg`
- `animate-elem-91-t.svg`
- `color-prop-03-t.svg`
- `conform-viewers-01-t.svg`
- `coords-trans-01-b.svg`
- `coords-trans-07-t.svg`

Result from `node ./scripts/compare-direct-port-spike.mjs /tmp/svgo-test-suite/svgo-test-suite /tmp/ferrovia-direct-port-spike-report`:

- baseline mismatches: `9 / 9`
- spike mismatches: `9 / 9`
- spike-only wins: `0`
- baseline-only wins: `0`

All files remained `both-mismatch`.

## Reading

This is not yet a go result.

The first spike iteration proves that the architectural seam is in place, but it does **not** yet show leverage over the stable baseline. In other words:

- the current compatibility layer is workable
- the current direct-port depth is not yet sufficient to move the hard residual hotspot set
- a larger transliteration effort would need to push deeper than just `cleanupIds` plus spike wrappers

## Decision

Current status: `No-Go Yet`

Reason:

- the spike does not beat the control baseline
- the hotspot remainder is not narrower yet
- the current result does not justify replacing the corpus-driven parity path

## Next Spike Options

1. Deepen the spike only if it ports the helper graph that actually drives the residual wall:
   - `convertPathData` helper flow and tie-break logic
   - selector/CSS tree behavior behind `inlineStyles`
   - shared reference and deopt analysis
2. Otherwise keep the spike as evidence and return to the main parity path.

Recommendation:

- keep the spike branch and tooling
- do not switch architecture yet
- only continue the spike if the next slice goes materially deeper into `convertPathData` helper behavior
