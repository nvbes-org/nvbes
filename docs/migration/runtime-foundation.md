# Runtime Foundation Ledger

## Status

- runtimes: 4
- pending: 0
- accepted: 0
- passed: 4
- failed: 0
- missing_paths: 0
- missing_commands: 0

## Rules

- Every runtime row must match the static runtime foundation contract.
- `go` requires all required paths and commands to be present.
- Missing paths, missing commands, evidence and proof must match generated repository state.
- Summary counters must match runtime rows.
- Strict cutover requires every runtime decision to be `go`.
- Generation provenance must identify write and strict cutover commands.

## Runtimes

| Runtime | Status | Decision | Missing Paths | Missing Commands | Owner | Proof |
|---|---:|---:|---:|---:|---|---|
| Rust | passed | go | 0 | 0 | Platform lead | `pnpm check:api` |
| TypeScript | passed | go | 0 | 0 | Platform lead | `pnpm check:web` |
| Go | passed | go | 0 | 0 | Platform lead | `pnpm check:go` |
| Python | passed | go | 0 | 0 | Platform lead | `pnpm check:python` |

## Regeneration

```bash
pnpm check:migration-runtime-foundation
node tools/migration/runtime-foundation.mjs --write
```
