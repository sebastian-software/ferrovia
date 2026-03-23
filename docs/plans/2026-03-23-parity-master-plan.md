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
- First reproducible `smoke-20` triage result:
  - `20 / 20` mismatches
  - `foreign-descriptive-subtree-retained`: `20`
  - `serializer-quote-normalization`: `19`

## Active Cluster Backlog
1. `foreign-descriptive-subtree-retained`
   - Symptom: Ferrovia preserves XHTML child content under foreign namespaced W3C description elements such as `d:testDescription`, while SVGO strips the unknown subtree and keeps the container element.
   - Expected owner: `removeUnknownsAndDefaults`
   - Status: In progress
2. `serializer-quote-normalization`
   - Symptom: Ferrovia preserves input single quotes more often than SVGO, which tends to serialize canonical double-quoted attributes.
   - Expected owner: serializer normalization
   - Status: Identified
3. `transform-folding-and-shape-normalization`
   - Symptom: Some W3C animation files still differ because SVGO folds transforms or shape geometry further than Ferrovia.
   - Expected owner: geometry / serializer interaction
   - Status: Identified
4. `namespace-and-reference-cleanup`
   - Symptom: Namespace removal and reference tracking still need broader corpus validation beyond the already fixed detached-subtree case.
   - Expected owner: `removeUnusedNS` and shared reference helpers
   - Status: Open

## First Execution Block
- Harden `scripts/triage-svgo-corpus.mjs` into a reproducible artifact generator.
- Define corpus profiles in the shell wrapper.
- Close `foreign-descriptive-subtree-retained`.
- Remeasure `smoke-20` and `sample-100`, then update this file with the next dominant cluster.

## Commands
- Corpus gate:
  - `bash ./scripts/check-svgo-test-suite.sh /tmp/svgo-test-suite/svgo-test-suite smoke-20`
- Detailed triage:
  - `node ./scripts/triage-svgo-corpus.mjs /tmp/svgo-test-suite/svgo-test-suite/W3C_SVG_11_TestSuite/svg smoke-20`
