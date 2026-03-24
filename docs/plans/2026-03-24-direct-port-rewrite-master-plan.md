# Direct-Port Rewrite Master Plan

Date: 2026-03-24
Reference: `svgo@4.0.1`
Status: Active

## Goal

Rebuild `ferrovia-core` as a strict SVGO-shaped direct port while keeping the outer Rust, CLI, Node, and verification surfaces stable.

## Reset State

- Archive reference branch: `codex/svgo-direct-port-spike`
- Rewrite branch: `codex/svgo-direct-port-rewrite-reset`
- External interfaces preserved
- Old core implementation removed

## First Green Milestone

- workspace compat crates created
- new core tree created
- minimal parser, stringifier, xast helpers, and plugin dispatcher in place
- directly ported starter plugins:
  - `removeXMLProcInst`
  - `removeDoctype`
  - `removeComments`
  - `removeMetadata`
  - `removeTitle`
- CLI and Node smoke fixtures remain green on the `remove-comments` oracle case

## Current Progress

- `refactor: reset core to direct-port rewrite scaffold`
  - reset complete
  - minimal rewrite path is green
- `feat: add direct-port path style and tools modules`
  - `lib/path`, `lib/style`, and additional `lib/svgo/tools` helpers in place
- `feat: add direct-port helper plugin modules`
  - `plugins/_path`
  - `plugins/_transforms`
  - `plugins/applyTransforms`
  - attribute helpers on xast nodes to keep the direct-port surface close to SVGO
- `feat: add direct-port xast query layer`
  - `lib/xast` now supports a minimal real selector surface:
    - tag selectors
    - id selectors
    - class selectors
    - attribute presence/equality selectors
    - descendant and child combinators
  - `lib/svgo/css-select-adapter` now exposes parent, sibling, child, text, name, and attribute access over the rewrite xast tree
- `feat: add selector compat and first query-driven plugin`
  - `ferrovia-css-what-compat` now parses a minimal but real selector IR
  - `ferrovia-css-select-compat` now runs selector matching through a small adapter trait
  - `lib/xast` delegates selector parsing and matching to the compat crates
  - `removeAttributesBySelector` is now ported against that path
- `feat: port direct-port attribute cleanup plugins`
  - `removeAttrs` now mirrors the upstream pattern-driven attribute removal flow
  - `removeElementsByAttr` now mirrors the upstream id/class element pruning flow
  - query-driven cleanup plugins now cover selector, attribute, and id/class removal on the rewrite tree
- `feat: port direct-port simple cleanup plugins`
  - `removeDesc`
  - `removeDimensions`
  - `removeEditorsNSData`
  - `removeEmptyAttrs`
  - `removeEmptyText`
  - `_collections` now carries the minimal editor namespace and conditional-processing sets needed by those ports
- `feat: port direct-port removeEmptyContainers`
  - `_collections` now carries the minimal container element set needed by the direct port
  - `removeEmptyContainers` now removes empty non-svg containers, keeps pattern/mask/filter edge cases, and prunes `use` references to removed ids
- `feat: port direct-port raster/style cleanup plugins`
  - `removeRasterImages`
  - `removeStyleElement`
  - both ports stay file-local and add no new shared rewrite surface
- `feat: port direct-port removeDeprecatedAttrs`
  - `_collections` now carries the first explicit deprecated-attribute group and element metadata needed by a style-aware cleanup port
  - `removeDeprecatedAttrs` now mirrors the upstream special-case around `xml:lang`/`lang` and respects attribute selectors referenced from stylesheets
- `feat: port direct-port script/xml cleanup plugins`
  - `removeScripts`
  - `removeXMLNS`
  - `_collections` now carries the minimal event-attribute groups needed for script stripping
- `feat: port direct-port removeUnusedNS`
  - namespace cleanup now mirrors the upstream root-svg collection/removal flow for prefixed namespace declarations
- current topological follow-up after selector compat:
  - deepen selector coverage beyond the current minimal surface
  - port the next simple upstream cleanup plugins before the heavier style/geometry blocks
  - next: finish the namespace block with `removeXlink`

## Port Order

1. `lib/types`, `lib/util/visit`, `lib/util/map-nodes-to-parents`
2. compat crates needed by the next files
3. `lib/xast`, `lib/path`, `lib/svgo/tools`, `lib/style`
4. `lib/parser`, `lib/stringifier`
5. helper plugin files
6. plugins in upstream order
7. builtin/preset/svgo entry modules
