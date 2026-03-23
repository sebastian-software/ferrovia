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

## Active Cluster Backlog
1. `foreign-descriptive-subtree-retained`
   - Symptom: Ferrovia preserves XHTML child content under foreign namespaced W3C description elements such as `d:testDescription`, while SVGO strips the unknown subtree and keeps the container element.
   - Expected owner: `removeUnknownsAndDefaults`
   - Status: Nearly closed; one remaining `sample-100` occurrence (`animate-elem-82-t.svg`)
2. `serializer-quote-normalization`
    - Symptom: Ferrovia preserves input single quotes more often than SVGO, which tends to serialize canonical double-quoted attributes.
    - Expected owner: serializer normalization
   - Status: Nearly closed; one remaining `sample-100` occurrence (`conform-viewers-03-f.svg`)
3. `transform-folding-and-shape-normalization`
   - Symptom: Some W3C animation files still differ because SVGO folds transforms or shape geometry further than Ferrovia.
   - Expected owner: geometry / serializer interaction
   - Status: Reduced, but still open on `coords-trans-04/05/06-t.svg`
4. `unclassified`
   - Symptom: Remaining corpus mismatches that are no longer explained by the first closed W3C-description cluster or the current quote/transform heuristics.
   - Expected owner: next triage pass
   - Status: Dominant remainder; first manual inspection shows this is no longer one bucket but at least:
     - path command canonicalization and shorthand/implicit-command parity in `convertPathData`
     - conservative retention of W3C harness structure such as root `id`/`width`/`height`, `<defs><style>`, and descriptive wrappers
5. `namespace-and-reference-cleanup`
   - Symptom: Namespace removal and reference tracking still need broader corpus validation beyond the already fixed detached-subtree case.
   - Expected owner: `removeUnusedNS` and shared reference helpers
   - Status: Open

## First Execution Block
- Harden `scripts/triage-svgo-corpus.mjs` into a reproducible artifact generator.
- Define corpus profiles in the shell wrapper.
- Close `foreign-descriptive-subtree-retained`.
- Remeasure `smoke-20` and `sample-100`, then update this file with the next dominant cluster.
- Result: complete. The next explicit low-risk target was `serializer-quote-normalization`, and the next concrete follow-up after the transform work is to split `unclassified` into:
  - `path-command-canonicalization`
  - `w3c-harness-structure-retained`

## Commands
- Corpus gate:
  - `bash ./scripts/check-svgo-test-suite.sh /tmp/svgo-test-suite/svgo-test-suite smoke-20`
- Detailed triage:
  - `node ./scripts/triage-svgo-corpus.mjs /tmp/svgo-test-suite/svgo-test-suite smoke-20 /tmp/ferrovia-smoke20-post-transform`
  - `node ./scripts/triage-svgo-corpus.mjs /tmp/svgo-test-suite/svgo-test-suite sample-100 /tmp/ferrovia-sample100-post-transform`
