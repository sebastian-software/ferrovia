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
- current topological follow-up after xast/query parity:
  - deepen compat crates beyond stubs
  - port next plugin files in upstream order on top of the helper and query layers

## Port Order

1. `lib/types`, `lib/util/visit`, `lib/util/map-nodes-to-parents`
2. compat crates needed by the next files
3. `lib/xast`, `lib/path`, `lib/svgo/tools`, `lib/style`
4. `lib/parser`, `lib/stringifier`
5. helper plugin files
6. plugins in upstream order
7. builtin/preset/svgo entry modules
