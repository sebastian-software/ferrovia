# ADR 0006: Delivery And Commit Discipline

- Status: accepted
- Date: 2026-03-20

## Context

The project is expected to grow through many plugin waves and parity fixes. Large uncommitted changes would make it hard to recover from failed experiments or regressions.

## Decision

- Every clean milestone is committed immediately with a Conventional Commit message.
- A milestone is only commit-worthy when its relevant checks are green.
- Compatibility, lint, and packaging gates are part of the milestone definition, not optional cleanup.
- ADRs in `docs/adr/` are the canonical place to record durable architecture and process decisions.

## Consequences

- The git history should remain easy to bisect and easy to roll back.
- Process decisions are discoverable instead of living only in chat history.
- Future contributors can add new ADRs rather than re-litigating already accepted choices from scratch.
