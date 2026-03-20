# ADR 0007: Wave Ordering And Commit Strategy

- Status: accepted
- Date: 2026-03-20

## Context

The remaining SVGO port work is still large and spans very different risk classes. Some plugins are deterministic and mostly local rewrites, while others require broader tree mutation, style analysis, or geometry-specific submodels. The project needs an explicit execution order so plugin scope, fixtures, and commits stay predictable.

## Decision

- The next implementation block starts with the deterministic plugin wave.
- That wave is executed in this order:
  1. `sortAttrs`
  2. `sortDefsChildren`
  3. `removeUnusedNS`
  4. `removeUselessDefs`
- After that, work proceeds in this fixed sequence:
  1. structure and container plugins
  2. semantic defaults and hidden element plugins
  3. style and defs-adjacent plugins
  4. geometry and transform plugins
- Each plugin in the deterministic wave gets its own vendored upstream fixtures, green regression gates, and its own Conventional Commit.
- `preset-default` is only expanded with plugins that are already differential-test green.

## Consequences

- The history remains bisectable and rollback-friendly for each plugin.
- Low-risk deterministic behavior is captured first before more invasive tree rewrite and style work.
- The later waves are constrained by this order, reducing opportunistic drift and avoiding partial style or geometry systems too early.

