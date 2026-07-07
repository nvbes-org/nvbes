# Infra Deploy Evidence

## Status

- status: passed
- checks: 29
- passed: 29
- failed: 0

## Rules

- Every evidence row must be generated from the infra source contract.
- `passed` requires the configured file to contain the expected pattern.
- Summary counters must match evidence rows.
- Targeted tests must name the infra checks required by the cutover gate.
- Generation provenance must identify sources, write command and targeted tests.

## Evidence

| Check | Status | Path |
|---|---:|---|
| Helm chart is declared as an application chart | passed | `deploy/oss/helm/nvbes/Chart.yaml` |
| Helm values pin the OCI image registry | passed | `deploy/oss/helm/nvbes/values.yaml` |
| Helm values include Account Service image | passed | `deploy/oss/helm/nvbes/values.yaml` |
| Helm values include Cloud Service image | passed | `deploy/oss/helm/nvbes/values.yaml` |
| Kustomize references the Helm chart | passed | `deploy/oss/kustomize/kustomization.yaml` |
| Compose stack uses released OCI images | passed | `deploy/oss/compose/compose.yaml` |
| OSS OpenTofu boundary is documented | passed | `deploy/oss/opentofu/README.md` |
| Staging OpenTofu overlay declares staging environment | passed | `infrastructure/environments/staging/main.tf` |
| Staging overlay configures PostgreSQL backup retention | passed | `infrastructure/environments/staging/main.tf` |
| Staging overlay exposes required secret inventory | passed | `infrastructure/environments/staging/main.tf` |
| Staging edge enforces TLS settings | passed | `infrastructure/environments/staging/main.tf` |
| Staging outputs secret inventory without values | passed | `infrastructure/environments/staging/outputs.tf` |
| Staging README forbids committing real tfvars secrets | passed | `infrastructure/environments/staging/README.md` |
| PostgreSQL module enables encryption at rest | passed | `infrastructure/modules/scaleway-v1/database.tf` |
| PostgreSQL module enables managed backups | passed | `infrastructure/modules/scaleway-v1/database.tf` |
| Object storage module enables bucket versioning | passed | `infrastructure/modules/scaleway-v1/storage.tf` |
| Object storage module defines lifecycle rules | passed | `infrastructure/modules/scaleway-v1/storage.tf` |
| Runtime IAM policy grants secret manager access | passed | `infrastructure/modules/scaleway-v1/iam.tf` |
| Integration script validates development and staging OpenTofu | passed | `scripts/test-integration.sh` |
| Release gate runs staging smoke checks | passed | `scripts/release-gate.sh` |
| Release gate runs staging critical E2E checks | passed | `scripts/release-gate.sh` |
| Staging migration requires backup or restore point confirmation | passed | `scripts/migrate-staging.sh` |
| Staging smoke wrapper runs smoke and E2E | passed | `scripts/smoke-staging.sh` |
| Production observability runbook documents Alloy architecture | passed | `infrastructure/environments/production/README.md` |
| Production observability uses mounted secret files | passed | `infrastructure/environments/production/README.md` |
| Alloy pipeline redacts sensitive fields | passed | `infrastructure/environments/production/alloy.config.alloy` |
| Production Alloy compose has a readiness healthcheck | passed | `infrastructure/environments/production/docker-compose.observability.yml` |
| Backup/restore manifest enumerates restore evidence | passed | `docs/migration/backup-restore-manifest.md` |
| Observability manifest enumerates cutover evidence | passed | `docs/migration/observability-readiness.md` |

## Decision

Infra/deploy repository evidence is covered for OCI images, Helm, Kustomize, Compose, OpenTofu, secret inventory, backups and observability. Production cutover still requires the G5 strict staging rebuild, restore and rollback evidence.

## Regeneration

```bash
pnpm check:migration-infra-deploy
node tools/migration/infra-deploy.mjs --write
```
