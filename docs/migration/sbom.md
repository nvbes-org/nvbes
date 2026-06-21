# SBOM Coverage Manifest

## Status

- entries: 5
- complete: 5
- pending: 0

## Components

| Ecosystem | Manifest | Lockfile | Complete |
|---|---|---|---:|
| node | `package.json` | `pnpm-lock.yaml` | true |
| rust | `Cargo.toml` | `Cargo.lock` | true |
| go | `go.mod` | `go.mod` | true |
| python | `pyproject.toml` | `pyproject.toml` | true |
| containers | `deploy/oss/helm/nvbes/Chart.yaml` | `deploy/oss/helm/nvbes/values.yaml` | true |

## Regeneration

```bash
pnpm check:sbom
tools/security/sbom.mjs --write
```
