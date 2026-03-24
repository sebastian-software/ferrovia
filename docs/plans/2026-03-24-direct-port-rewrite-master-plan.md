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
- `feat: port direct-port removeXlink`
  - xlink cleanup now mirrors the upstream prefix-stack behavior, href/title/show conversion, and legacy-element deopt path
- `feat: port direct-port removeViewBox`
  - the first viewBox cleanup port is now in place for `svg`/`symbol`/`pattern` with the upstream nested-svg guard
- `feat: port direct-port removeOffCanvasPaths`
  - the first geometry-aware canvas cleanup port is now in place with root-viewBox parsing, transform subtree deopt, and conservative path/viewBox intersection checks
- `feat: port direct-port convertShapeToPath`
  - the first geometry-conversion port is now in place for `rect`, `line`, `polyline`, and `polygon`
  - optional arc conversion for `circle` and `ellipse` is now wired through the direct-port parameter surface
  - direct-port geometry coverage now reaches beyond cleanup into shape-to-path rewriting without introducing a ferrovia-specific abstraction layer
- `feat: port direct-port convertEllipseToCircle`
  - non-eccentric `ellipse` conversion now mirrors the upstream `rx === ry || auto` rule
  - the SVG2 `auto` radius cases are now handled on the same direct-port path
  - geometry rewriting now covers both direct shape-to-path conversion and the small ellipse-to-circle normalization pass
- `feat: port direct-port cleanupAttrs`
  - attribute value cleanup now mirrors the upstream newline, trim, and repeated-space pass
  - the direct port preserves the upstream parameter surface: `newlines`, `trim`, and `spaces`
  - recursive attribute cleanup is now covered without adding any new shared rewrite abstraction
- `feat: port direct-port cleanupEnableBackground`
  - root filter detection plus the `svg`/`mask`/`pattern` cleanup rules are now mirrored on the rewrite tree
  - the direct port now removes redundant `enable-background` attributes and their inline-style declaration form
  - style handling stays local to the plugin and does not yet force a broader css-tree rewrite surface
- `feat: tighten direct-port script detection helpers`
  - `lib/svgo/tools` now mirrors the upstream `hasScripts` checks for script content, javascript hrefs, and event attributes
  - the cleanupIds deopt surface now has the minimum upstream-compatible foundation it expects
- `feat: port direct-port cleanupIds`
  - the rewrite tree now mirrors the upstream duplicate-id cleanup, unused-id removal, and referenced-id minification flow
  - reference rewrites now cover `href`, `url(#...)`, and SMIL `begin` patterns on the direct-port path
  - the upstream deopt around embedded styles and scripts is now enforced on the rewrite path
- `feat: port direct-port collapseGroups`
  - post-order group collapsing now mirrors the upstream exit-phase behavior on the rewrite tree
  - the direct port now covers attribute migration into a single child, animation-aware deopts, and empty-group splicing
  - the minimal collection surface now includes animation elements and inheritable attributes needed by this rewrite block
- `feat: port direct-port moveGroupAttrsToElems`
  - group-level transform lifting now mirrors the upstream guarded move onto path/text/group children
  - url-reference and child-id deopts are now enforced on the rewrite path
  - the collection surface now includes the upstream `pathElems` distinction needed by this transform rewrite
- `feat: port direct-port moveElemsAttrsToGroup`
  - common inheritable child attributes now hoist back onto `g` elements through the upstream-style post-order rewrite flow
  - `transform` hoisting now respects the upstream deopts around all-path children, filter-like group attributes, and stylesheet presence
  - the direct port now covers both directions of the paired group-attribute movement rewrites before entering the heavier style layer
- `feat: port direct-port mergeStyles`
  - style-element merging now mirrors the upstream `foreignObject` skip, invalid-type deopt, empty-style removal, media wrapping, and CDATA promotion flow
  - the first style element is now kept as the merge anchor and later style elements are detached in the same direct-port order as SVGO
  - the rewrite has now entered the heavier style layer without replacing the current xast tree or serializer
- `feat: port direct-port inlineStyles`
  - style inlining now mirrors the first real upstream flow from stylesheet collection into matched element `style` attributes
  - the direct-port slice now covers selector matching, `onlyMatchedOnce`, matched-selector cleanup, class/id cleanup, and `foreignObject` skipping
  - CSS support is still intentionally minimal and targeted at the current rewrite surface, but the rewrite now has both style merging and style inlining in place before `minifyStyles`
- `feat: port direct-port minifyStyles`
  - the rewrite now mirrors the upstream gather-and-exit flow for `<style>` elements and `style` attributes
  - a minimal `ferrovia-csso-compat` minifier is now in place for style-element text and declaration-list minification, including empty-style removal and CDATA preservation for angle-bracket content
  - usage tracking and script deopt collection now exist on the direct-port path even though the compat minifier still uses only a minimal subset of that data
- current topological follow-up after selector compat:
  - the next upstream-missing files are now dominated by the remaining style and geometry blocks
  - the next reasonable direct-port jump is now back into the deeper cleanup/path blocks, especially `removeUnknownsAndDefaults`, `removeHiddenElems`, `convertColors`, `convertPathData`, and `mergePaths`
  - selector and CSS support can stay incremental until a later style-oriented plugin forces broader css-tree- and selector-surface parity than the current direct-port slice

## Port Order

1. `lib/types`, `lib/util/visit`, `lib/util/map-nodes-to-parents`
2. compat crates needed by the next files
3. `lib/xast`, `lib/path`, `lib/svgo/tools`, `lib/style`
4. `lib/parser`, `lib/stringifier`
5. helper plugin files
6. plugins in upstream order
7. builtin/preset/svgo entry modules
