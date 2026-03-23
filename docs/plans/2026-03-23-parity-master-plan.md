# Ferrovia Parity Master Plan

Date: 2026-03-23
Reference: `svgo@4.0.1`
Status: Active

## Goal
- Reach broad, measured parity with `svgo@4.0.1`, not only `preset-default` feature completeness.
- Keep `preset-default` as the hard baseline while using real `svgo-test-suite` mismatches to drive the remaining work.
- Close parity gaps cluster-by-cluster, with reproducible measurements and stored diff artifacts.

## Working Rules
- No new parity work lands without an identified fixture or corpus mismatch.
- Every milestone commit stays green on:
  - `cargo test`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test -p ferrovia-core --test upstream`
  - `bash ./scripts/check-regressions.sh`
- Every cluster fix is remeasured on:
  - `smoke-20`
  - `sample-100`
- Larger waves also run:
  - `milestone-500`

## Measurement Profiles
- `smoke-20`: first 20 sorted corpus files
- `sample-100`: first 100 sorted corpus files
- `milestone-500`: first 500 sorted corpus files
- `full-corpus`: all sorted corpus files

## Workflow
1. Run a corpus profile against the pinned SVGO oracle.
2. Use the triage tool to save `input`, `ferrovia`, `svgo`, `diff`, and `meta` artifacts for each mismatch.
3. Group mismatches into concrete clusters.
4. Fix the dominant cluster only.
5. Rerun the same measurement window and update this document.

## Current State
- `preset-default` is implemented and wired end-to-end.
- Local preset/default fixture gates are green.
- First corpus sampling still shows a dense W3C mismatch wall, especially in `W3C_SVG_11_TestSuite/svg/animate-*`.
- Baseline before the first corpus-driven fix:
  - `smoke-20`: `20 / 20` mismatches
  - dominant cluster: `foreign-descriptive-subtree-retained` on all 20 files
- Current state after closing the first `removeUnknownsAndDefaults` W3C description fix:
  - `smoke-20`: `18 / 20` mismatches
  - `sample-100`: `83 / 100` mismatches
  - `foreign-descriptive-subtree-retained`: reduced to `1` in `sample-100`
  - `serializer-quote-normalization`: `14` in `sample-100`
  - `transform-folding-and-shape-normalization`: `4` in `sample-100`
- Current state after serializer quote normalization:
  - `smoke-20`: `17 / 20` mismatches
  - `sample-100`: `80 / 100` mismatches
  - `serializer-quote-normalization`: reduced to `11` in `sample-100`
  - `transform-folding-and-shape-normalization`: still `4` in `sample-100`
  - `unclassified`: still the dominant remainder and the next investigation target
- Current state after text-node quote/entity escaping:
  - `smoke-20`: `16 / 20` mismatches
  - `sample-100`: `79 / 100` mismatches
  - quote-heavy text serialization cases are reduced again
  - the remaining dominant work is now clearly outside the simple serializer quote block
- Current state after affine transform baking for non-arc paths in `convertPathData`:
  - `smoke-20`: `11 / 20` mismatches
  - `sample-100`: `72 / 100` mismatches
  - `transform-folding-and-shape-normalization`: reduced to `3` in `sample-100`
  - `serializer-quote-normalization`: reduced to `1` in `sample-100`
  - the dominant remainder is now a broad `unclassified` W3C animation block, but the first inspected files split into two clearer subthemes:
    - `convertPathData` command-form parity, for example `animate-elem-04-t.svg` where SVGO prefers `m90 258 300-78` and collapsed implicit `M/L` syntax more aggressively than Ferrovia
    - conservative retention of test scaffolding and root metadata, for example `animate-elem-90-b.svg` where Ferrovia still keeps root `id`/`width`/`height`, `<defs><style>`, and other W3C harness structure that SVGO removes
- Current state after corpus-driven style/deopt cleanup, selective anchor-whitespace preservation, and deterministic `cleanupIds` ordering:
  - `smoke-20`: `9 / 20` mismatches
  - `sample-100`: `65 / 100` mismatches
  - `foreign-descriptive-subtree-retained`: still `1` in `sample-100`
  - `serializer-quote-normalization`: still `1` in `sample-100`
  - `transform-folding-and-shape-normalization`: still `3` in `sample-100`
  - the latest closed causes in this block were:
    - detached `<style>` nodes no longer deoptimize cleanup/minification passes after they have been removed from the live tree
    - anchor-local whitespace text survives cleanup without globally reintroducing whitespace-only text nodes into tree-rewrite plugins
    - `cleanupIds` now minifies in encounter order, matching SVGO on animation reference cases such as `animate-elem-20-t.svg`
  - the dominant remainder is now even more clearly a `convertPathData` and animation-structure block:
    - path command canonicalization and implicit-command parity in `animate-elem-04/05/06/07/08-t.svg`
    - structural attribute / group normalization around translated animation scaffolds in `animate-elem-09/10/11/12-t.svg`
- Current state after closing the first W3C animation canonicalization wave:
  - `smoke-20`: `0 / 20` mismatches
  - `sample-100`: `46 / 100` mismatches
  - the major closed causes in this wave were:
    - `convertPathData` now matches SVGO more closely on implicit `M/L` compaction, initial relative `m` preference, smooth-curve shorthand selection, and affine transform bake-in for non-arc paths
    - `moveGroupAttrsToElems` now repeats until nested groups created in the same pass receive the expected propagated transforms
    - `removeUnknownsAndDefaults` now drops default `text[x=0]` / `text[y=0]` in the same way as SVGO for the translated W3C animation scaffolds
  - new direct regressions now pin:
    - repeated-curve serialization compaction for `convertPathData`
    - `cleanupIds` begin-list spacing preservation
    - mixed-content indentation trimming in SVG text containers
  - the remaining `sample-100` wall is no longer a generic W3C animation bucket; it splits into a few concrete clusters with clear ROI ordering:
    - SMIL- and default-value normalization drift
    - mixed-content / text / script serialization
    - `convertPathData` and path canonicalization drift
    - transform bake-in and geometry rewrite drift
    - residual default-/inherit-materialization cases such as `stop-color="inherit"`
- Current state after the first SMIL/default normalization wave:
  - `smoke-20`: `0 / 20` mismatches
  - `sample-100`: `35 / 100` mismatches
  - the major closed causes in this wave were:
    - `removeUnknownsAndDefaults` now treats `use[x|y=0]` and `image[x|y=0]` as removable defaults, matching SVGO on the W3C animation harness files
    - `stop-color` and `stop-opacity` now participate in inherited-style cleanup, so inherited `stop-*="inherit"` overrides no longer survive unnecessarily on child `<stop>` nodes
    - `set` and the other SVG animation elements now have a minimal attribute model, which strips unsupported event attrs such as `onend` from `<set>` without destabilizing valid timing/target attrs
  - the cluster shape has shifted again:
    - the old broad `smil-and-default-normalization` block is materially smaller
    - the highest-value remainder inside that area is now narrower and more concrete: `cleanupIds` still over-rewrites some SMIL syncbase / repeat / event-base references in `begin=` lists
    - representative file: `animate-elem-61-t.svg`, where SVGO preserves names like `syncBase.begin`, `repeatBase.repeat(4)`, and `setFourTarget.click+4s`, while Ferrovia still rewrites them to minified local ids
  - the remaining wall is now dominated even more clearly by:
    - SMIL reference preservation in `begin=` and related timing expressions
    - mixed-content / text / script serialization
    - `convertPathData` and path canonicalization drift
    - transform bake-in and geometry rewrite drift
- Current state after tightening `cleanupIds` begin-expression rewrites:
  - `smoke-20`: `0 / 20` mismatches
  - `sample-100`: `34 / 100` mismatches
  - closed cause in this slice:
    - `cleanupIds` now rewrites only the first matching SMIL `begin=` segment for a given renamed ID, which matches SVGO on files such as `animate-elem-61-t.svg`
  - the high-value remainder is unchanged in shape, but now even narrower:
    - mixed-content / text / script serialization
    - `convertPathData` and path canonicalization drift
    - transform bake-in and geometry rewrite drift
- Current state after mixed-content and script serializer normalization:
  - `smoke-20`: `0 / 20` mismatches
  - `sample-100`: `32 / 100` mismatches
  - closed causes in this slice:
    - mixed `text` containers now keep SVGO-like source-faithful line structure instead of being flattened into a single line
    - `script` text now trims only outer indentation while preserving inner line structure, which closes a chunk of the W3C script/mixed-content drift
  - the remaining wall is now even more concentrated in:
    - `convertPathData` and path canonicalization drift
    - transform bake-in and geometry rewrite drift
    - a small residual serializer/reference tail
- Current state after the first focused path-canonicalization slice:
  - `smoke-20`: `0 / 20` mismatches
  - `sample-100`: `28 / 100` mismatches
  - closed causes in this slice:
    - `convertPathData` now rebuilds smooth cubic shorthands from reflected control points instead of only handling the trivial `0,0`/cursor case
    - `convertPathData` now chooses absolute quadratic commands when they serialize shorter, which closes the alternating `q/Q` drift in W3C animation shapes
    - redundant explicit line-backtracking to the subpath start immediately before `z` is now removed, which matches SVGO on zero-height/closepath forms such as `h25z`
  - representative wins:
    - `animate-elem-28-t.svg`
    - `animate-elem-37-t.svg`
    - the `convertPathData` part of `animate-elem-32/34-t.svg`
  - the remaining wall has shifted again:
    - the old path-canonicalization block is materially smaller
    - the dominant remainder is now mostly broader geometry / transform / structure drift plus a large `unclassified` tail outside the simple command-form mismatches
- Current state after preserving mixed-content whitespace nodes in `text`-like containers:
  - `smoke-20`: `0 / 20` mismatches
  - `sample-100`: `23 / 100` mismatches
  - closed causes in this slice:
    - parser and serializer now keep whitespace-only text nodes inside `text`-like mixed-content containers instead of discarding them as globally empty
    - SVGO-like line structure is now preserved between text payload and animation children such as `animateMotion`, `animateTransform`, and `set`
  - representative wins:
    - `animate-elem-32/34/36/44-t.svg`
    - `animate-elem-60/62/77/78/80/81/91-t.svg`
    - the mixed-content portion of `animate-elem-24/30-t.svg`
  - the remaining wall is now narrower and more technical:
    - numeric / transform precision drift, for example `rotate(-15 721.118 -194.84)` vs `rotate(-15 721.118 -194.841)` in `animate-elem-24-t.svg`
    - path canonicalization drift, for example `m95 40 20 20-20 20-20-20z` vs `m95 40 20 20L95 80 75 60z` in `color-prop-03-t.svg`
    - merge-paths / structural grouping drift, for example `coords-trans-01-b.svg`
    - one remaining foreign-description case and one animated gradient/default-style case
- Current state after smart numeric shortening within the configured precision window:
  - `smoke-20`: `0 / 20` mismatches
  - `sample-100`: `22 / 100` mismatches
  - closed causes in this slice:
    - number serialization now prefers shorter values whenever the shorter form stays within the active precision tolerance instead of always materializing the full rounded decimal
    - this closes the simple transform / geometry tail where Ferrovia produced values like `-194.841` or `43.301` while SVGO chose `-194.84` and `43.3`
  - representative wins:
    - `animate-elem-24-t.svg`
    - the numeric-only portion of `coords-trans-07-t.svg`
  - the remaining wall is now even more concentrated:
    - transform / shape normalization drift, for example `translate(40)scale(.8)` vs `matrix(.8 0 0 .8 40 0)` in `animate-elem-44-t.svg`
    - path canonicalization drift, for example the remaining transformed polygon / guide-path cases in `animate-elem-30/36/82-t.svg` and `color-prop-03-t.svg`
    - merge-paths / structural grouping drift, for example `coords-trans-01-b.svg`
    - one remaining foreign-description classifier miss and one animated gradient/default-style case

## Active Cluster Backlog
1. `smil-reference-preservation`
   - Symptom: Ferrovia still rewrites some SMIL syncbase, repeat, and event-base references inside `begin=` expressions more aggressively than SVGO.
   - Typical diffs:
     - `syncBase.begin + 4s` rewritten to `b.begin + 4s`
     - `repeatBase.repeat(4)` rewritten to `c.repeat(4)`
     - `setFourTarget.click+4s` rewritten to `d.click+4s`
   - Representative files:
     - `animate-elem-77/78-t.svg`
   - Expected owner: `cleanupIds` begin-expression analysis and selective reference rewriting
   - Status: Mostly closed in `sample-100`; remaining tail is no longer the main blocker
2. `mixed-content-serialization`
   - Symptom: Ferrovia still serializes some `text`/`script`/mixed-content blocks differently from SVGO.
   - Typical diffs:
     - inline animate/set children pulled tighter or looser than SVGO
     - script blocks and mixed-content text containers preserving layout indentation differently
   - Representative files:
     - `animate-elem-24-t.svg`
     - `animate-interact-pevents-01-t.svg`
     - `animate-script-elem-01-b.svg`
     - `animate-struct-dom-01-b.svg`
   - Expected owner: serializer normalization
   - Status: materially reduced; no longer the top blocker
3. `path-canonicalization`
  - Symptom: `convertPathData` still chooses different absolute/relative forms or path shorthands than SVGO.
  - Typical diffs:
    - `s` vs `c`
    - `Q/q` choice drift
    - different command grouping or shape-to-path output forms
    - relative line runs converted to absolute `L` even when SVGO keeps the compact relative form
  - Representative files:
    - `color-prop-03-t.svg`
    - `conform-viewers-01-t.svg`
  - Expected owner: `convertPathData`, shape conversion, and path serializer interaction
  - Status: Partially closed; remaining tail now overlaps more with transform/shape rewriting than with simple shorthand selection
4. `transform-and-geometry-rewrite`
  - Symptom: Ferrovia bakes transforms or rewrites geometry in places where SVGO leaves the structural transform/shape form intact.
  - Representative files:
     - `animate-elem-24-t.svg`
     - `coords-trans-07-t.svg`
   - Expected owner: transform gating and geometry normalization
   - Status: Open and now one of the highest-ROI remaining causes
5. `foreign-descriptive-subtree-retained`
  - Symptom: One remaining foreign-description case still retains XHTML child content that SVGO strips.
  - Representative file:
     - `animate-elem-82-t.svg`
   - Expected owner: `removeUnknownsAndDefaults`
   - Status: Nearly closed; one known `sample-100` occurrence
6. `animated-gradient-defaults`
   - Symptom: Ferrovia still drops animated inherited/default presentation values on gradient wrapper groups that SVGO keeps.
   - Representative file:
     - `animate-pservers-grad-01-b.svg`
   - Expected owner: `removeUnknownsAndDefaults` and inherited-style cleanup around animated `stop-color`
   - Status: One known `sample-100` occurrence
7. `serializer-quote-normalization`
   - Symptom: One remaining file still differs only by quote/attribute normalization.
   - Representative file:
     - `conform-viewers-03-f.svg`
   - Expected owner: serializer normalization
   - Status: Nearly closed; one known `sample-100` occurrence

## First Execution Block
- Harden `scripts/triage-svgo-corpus.mjs` into a reproducible artifact generator.
- Define corpus profiles in the shell wrapper.
- Close `foreign-descriptive-subtree-retained`.
- Remeasure `smoke-20` and `sample-100`, then update this file with the next dominant cluster.
- Result: complete. The next explicit low-risk target was `serializer-quote-normalization`, and the next concrete follow-up after the transform work is to split `unclassified` into:
  - `path-command-canonicalization`
  - `translated-animation-scaffold-normalization`
  - `remaining-w3c-harness-structure-retained`

## Next Execution Block
- Continue the remaining `path-canonicalization` tail only where the diff is still command-form driven.
- Keep scope tight:
  - continue on corpus-proven `convertPathData` drift only where it is still about command-form parity
  - prioritize the residual `conform-viewers-01-t.svg`-style degenerations and curve-to-line reductions if they are still isolated and high ROI
  - then switch to the broader transform/geometry block and translated W3C structure drift, which now dominate the remaining sample mismatches more than the simple shorthand cases
- Remeasure:
  - `smoke-20`
  - `sample-100`
- Only after that continue with the broader transform/geometry block and the small residual serializer/reference tail.

## Commands
- Corpus gate:
  - `bash ./scripts/check-svgo-test-suite.sh /tmp/svgo-test-suite/svgo-test-suite smoke-20`
- Detailed triage:
  - `node ./scripts/triage-svgo-corpus.mjs /tmp/svgo-test-suite/svgo-test-suite smoke-20 /tmp/ferrovia-smoke20-post-transform`
  - `node ./scripts/triage-svgo-corpus.mjs /tmp/svgo-test-suite/svgo-test-suite sample-100 /tmp/ferrovia-sample100-post-transform`
