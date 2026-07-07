# Supply Chain Control Ledger

## Status

- entries: 5
- active: 5
- pending: 0

## Rules

- Every control row must match the static supply-chain contract.
- `active` requires the package script and evidence path to exist.
- Summary counters must match generated rows.
- Strict cutover requires every control to be active.
- Generation provenance must identify write and strict cutover commands.

## Controls

| Control | Status | Command | Evidence |
|---|---:|---|---|
| secrets | active | `pnpm check:secrets` | `tools/security/scan-secrets.mjs` |
| licenses | active | `pnpm check:oss-licenses` | `scripts/check-oss-licenses.mjs` |
| dependencies | active | `pnpm check:dependencies` | `tools/security/check-dependencies.mjs` |
| sbom | active | `pnpm check:sbom` | `docs/migration/sbom.generated.json` |
| containers | active | `pnpm check:containers` | `tools/security/check-containers.mjs` |

## Regeneration

```bash
pnpm check:migration-supply-chain
node tools/migration/supply-chain.mjs --write
```
