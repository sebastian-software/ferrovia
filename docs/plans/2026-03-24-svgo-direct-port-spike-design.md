# SVGO Direct-Port Spike Design

Date: 2026-03-24
Reference: `svgo@4.0.1`
Status: Active

## Current Outcome

- Initial comparison report: [2026-03-24-svgo-direct-port-spike-report.md](/Users/sebastian/Workspace/ferrovia/docs/plans/2026-03-24-svgo-direct-port-spike-report.md)
- Current decision state: `No-Go Yet`

## Goal

Validate whether a JS-structure-near semantic transliteration closes the remaining parity wall faster than the current Rust-first parity repair path.

## Baseline

- Stable baseline branch state: `sample-100 = 18 / 100`
- Stable baseline branch state: `smoke-20 = 0 / 20`
- Current baseline remains the control group for all spike measurements

## Scope

The spike covers exactly three plugin families plus their minimal helper graph:

1. `cleanupIds`
2. `convertPathData`
3. `inlineStyles`

The spike does not replace:

- parser
- arena document model
- serializer
- CLI or Node public API

## Approach

Recommended approach: keep the ferrovia tree as the source of truth, but add a thin compatibility layer that lets selected plugin ports follow SVGO-shaped traversal and helper flow.

### Alternatives Considered

1. Continue only with the current corpus-driven path.
   - Pro: no duplicate implementation styles
   - Con: remaining parity wall is too sensitive to hidden helper ordering and tie-break behavior

2. Full rewrite around a new DOM and query engine.
   - Pro: could resemble SVGO more directly
   - Con: too large, too risky, and destroys the value of the current stable baseline

3. Selected direct-port spike on the current tree.
   - Pro: isolates the strategic question while reusing the stable parser and serializer
   - Con: temporary overlap with existing ferrovia-native logic

Chosen: option 3.

## Module Mapping

### Parser / tokenizer boundary

- Upstream:
  - `lib/parser.js`
  - `lib/xast.js`
- Spike destination:
  - keep current parser unchanged
  - add `svgo_spike::compat` traversal and query helpers over the existing `Document`

### Query / xast boundary

- Upstream:
  - `lib/xast.js`
  - `css-select`
- Spike destination:
  - `svgo_spike::compat`
  - use current selector matching as the query backend
  - expose `query_selector_all`, `walk_elements`, `has_scripts`, and reference scanning in an SVGO-shaped way

### Style / CSS helper boundary

- Upstream:
  - `lib/style.js`
  - `plugins/inlineStyles.js`
  - `css-tree`, `css-what`, `csso`
- Spike destination:
  - `svgo_spike::inline_styles`
  - current `style.rs` remains the CSS parser/serializer backend for the spike

### Path / transform helper boundary

- Upstream:
  - `plugins/convertPathData.js`
  - `plugins/_path.js`
  - `plugins/applyTransforms.js`
- Spike destination:
  - `svgo_spike::convert_path_data`
  - current `geometry.rs` and existing path conversion logic remain the initial backend

## Translation Matrix

| Plugin family | Upstream source | Rust destination | Shared helpers | External crates | Known semantic traps |
| --- | --- | --- | --- | --- | --- |
| `cleanupIds` | `plugins/cleanupIds.js`, `lib/svgo/tools.js` | `svgo_spike::cleanup_ids` | `compat::find_references`, `compat::has_scripts`, attribute rewrite helpers | none new | style/script deopt, defs-only skip, duplicate IDs, SMIL `begin=` rewrites |
| `inlineStyles` | `plugins/inlineStyles.js`, `lib/xast.js`, `lib/style.js` | `svgo_spike::inline_styles` | `compat::query_selector_all`, current `style.rs` parsers | none new | `foreignObject` skip, selector specificity order, MQ filtering, attr cleanup after inlining |
| `convertPathData` | `plugins/convertPathData.js`, `plugins/_path.js`, `plugins/applyTransforms.js` | `svgo_spike::convert_path_data` | current path/transform helpers plus spike orchestration | `svgtypes` already present | transform application gate, stroke and marker deopts, absolute/relative tie-breaks |

## Acceptance Corpus

The spike report must cover these files at minimum:

- `animate-elem-60-t.svg`
- `animate-elem-62-t.svg`
- `animate-elem-78-t.svg`
- `animate-elem-83-t.svg`
- `animate-elem-91-t.svg`
- `color-prop-03-t.svg`
- `conform-viewers-01-t.svg`
- `coords-trans-01-b.svg`
- `coords-trans-07-t.svg`

For each file the report stores:

- baseline ferrovia output
- spike output
- SVGO oracle output
- normalized diff
- mismatch cause label

## Success Criteria

The spike is a go if at least one of these is true:

- `sample-100` beats the stable baseline `18 / 100`
- the hotspot corpus mismatch count drops by at least `50%`
- the hotspot remainder narrows to a smaller and more clearly attributable helper surface

Otherwise the spike is a no-go and the existing parity path remains primary.

## Execution Plan

1. Freeze the baseline in docs and branch workflow.
2. Add the compatibility layer and spike entrypoint.
3. Port `cleanupIds`, `inlineStyles`, and `convertPathData` as SVGO-shaped spike modules.
4. Add a comparison runner for baseline vs spike vs oracle.
5. Produce a decision report from the hotspot corpus and `sample-100`.
