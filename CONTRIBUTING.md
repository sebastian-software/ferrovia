# Contributing

Thanks for contributing to `ferrovia`.

## Before You Start

- Open an issue for larger changes before investing in a broad implementation.
- Keep scope small and easy to review.
- Preserve existing behavior unless the change explicitly targets a verified compatibility fix.

## Development Workflow

1. Start from a clean branch.
2. Make one logical change at a time.
3. Keep commits small and reversible.
4. Use Conventional Commits for commit messages.

Examples:

- `feat: port removeEmptyContainers plugin`
- `fix: preserve mask ids in removeEmptyContainers`
- `test: add oracle coverage for moveElemsAttrsToGroup`

## Verification

Run the relevant checks before submitting changes:

```bash
cargo test
cargo clippy --all-targets --all-features -- -D warnings
bash ./scripts/check-regressions.sh
```

If your change touches Node bindings or packaging, also verify:

```bash
pnpm build:napi
pnpm test:node
```

## Project Conventions

- Favor fixture- and oracle-driven validation over inferred compatibility claims.
- Add or update ADRs in `docs/adr/` for durable architectural decisions.
- Do not mix unrelated refactors into plugin parity work.

## Pull Requests

- Explain user-visible or compatibility-relevant behavior changes clearly.
- Call out any known deviations from upstream SVGO.
- Include follow-up items when a change is intentionally partial.
