# Codegen and SDK Control Ledger

## Status

- entries: 6
- active: 6
- pending: 0

## Rules

- Control rows must match the static codegen contract.
- Summary counters must match generated control rows.
- Generation provenance must identify write and strict cutover commands.

## Controls

| Control | Status | Command | Evidence |
|---|---:|---|---|
| openapi | active | `pnpm check:contracts` | `contracts/openapi/manifest.json` |
| protobuf | active | `pnpm check:contracts` | `contracts/protobuf/nvbes/platform/v1/common.proto` |
| events | active | `pnpm check:contracts` | `contracts/events/manifest.json` |
| typescript-sdk | active | `pnpm check:codegen` | `libs/ts/identity-sdk-core/src/types.gen.ts` |
| rust-sdk | active | `pnpm check:codegen` | `libs/rust/identity-sdk-backend/src/lib.rs` |
| go-sdk | active | `pnpm check:codegen` | `libs/go/identity-sdk/sdk.go` |

## Regeneration

```bash
pnpm check:migration-codegen
node tools/migration/codegen.mjs --write
```
