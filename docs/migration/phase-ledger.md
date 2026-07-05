# Migration Phase Ledger

## Status

- phases: 13
- pending: 3
- accepted: 0
- passed: 10
- failed: 0

## Phases

| Phase | Status | Decision | Owner | Evidence | Proof |
|---|---:|---:|---|---|---|
| P01 Freeze et inventaire | passed | go | Migration lead | `docs/migration/inventory.generated.json`<br>`docs/migration/data-map.generated.json`<br>`docs/migration/secret-map.generated.json`<br>`docs/migration/job-map.generated.json`<br>`docs/migration/resource-map.generated.json` | `pnpm check:migration-inventory && pnpm check:migration-data-map && pnpm check:migration-secret-map && pnpm check:migration-job-map && pnpm check:migration-resource-map` |
| P02 Nouvelle fondation | passed | go | Platform lead | `docs/migration/target-structure.generated.json`<br>`docs/migration/codegen.generated.json`<br>`docs/migration/supply-chain.generated.json`<br>`docs/migration/runtime-foundation.generated.json`<br>`tools/boundary-checks/check-product-boundaries.mjs`<br>`scripts/check-nx-boundaries.mjs` | `pnpm check:migration-target-structure && pnpm check:migration-codegen && pnpm check:migration-supply-chain && pnpm check:migration-runtime-foundation && pnpm check:product-boundaries && pnpm check:oss-boundaries` |
| P03 Primitives plateforme | passed | go | Platform lead | `docs/migration/platform-primitives.generated.json`<br>`libs/rust/platform/src/platform.outbox.rs`<br>`libs/rust/ports/src/lib.rs`<br>`libs/rust/audit/src/lib.rs`<br>`libs/rust/tenancy/src/lib.rs`<br>`libs/rust/core/src/http.error.rs`<br>`libs/rust/observability/src/lib.rs` | `pnpm check:migration-platform-primitives && cargo test -p nvbes-platform --locked && cargo test -p nvbes-ports --locked && cargo check --workspace --locked` |
| P04 Identity | passed | go | Identity lead | `docs/migration/identity-register.generated.json`<br>`docs/migration/identity-login-session.generated.json`<br>`docs/migration/identity-mfa-webauthn.generated.json`<br>`apps/identity-api/openapi.json` | `pnpm check:migration-identity-register && pnpm check:migration-identity-login-session && pnpm check:migration-identity-mfa-webauthn` |
| P05 Workspace/Authz | passed | go | Workspace lead | `docs/migration/workspace-membership-roles.generated.json`<br>`docs/migration/workspace-last-owner.generated.json`<br>`libs/rust/core/src/authz.policy.tests.rs` | `pnpm check:migration-workspace-membership-roles && pnpm check:migration-workspace-last-owner` |
| P06 Drive | passed | go | Drive lead | `docs/migration/drive-upload-download.generated.json`<br>`docs/migration/drive-share-revoke.generated.json`<br>`docs/migration/drive-quotas.generated.json`<br>`apps/drive-api/openapi.json` | `pnpm check:migration-drive-upload-download && pnpm check:migration-drive-share-revoke && pnpm check:migration-drive-quotas` |
| P07 Billing/Usage | passed | go | Billing lead | `docs/migration/billing-entitlements.generated.json`<br>`docs/migration/billing-webhook-idempotency.generated.json`<br>`docs/migration/billing-multi-psp-continuity.generated.json`<br>`libs/rust/billing/src/views.rs` | `pnpm check:migration-billing-entitlements && pnpm check:migration-billing-webhook-idempotency && pnpm check:migration-billing-multi-psp-continuity` |
| P08 Developer Platform | passed | go | Developer Platform lead | `docs/migration/developer-oauth-tokens.generated.json`<br>`docs/migration/developer-signed-webhooks.generated.json`<br>`apps/developer-web/src/developer.router.tsx`<br>`libs/ts/identity-sdk-core/openapi.json` | `pnpm check:migration-developer-oauth-tokens && pnpm check:migration-developer-signed-webhooks` |
| P09 Frontends | passed | go | Product frontend lead | `docs/migration/frontend-experience.generated.json`<br>`apps/identity-web/e2e/critical.spec.ts`<br>`apps/drive-web/src/drive.router.tsx`<br>`apps/developer-web/src/developer.router.tsx`<br>`apps/enterprise-web/src/enterprise.router.tsx`<br>`apps/cloud-console/README.md`<br>`apps/internal-admin/README.md` | `pnpm check:migration-frontend-experience && pnpm check:web && pnpm --dir apps/enterprise-web test` |
| P10 Infra/deploy | passed | go | Infra lead | `docs/migration/infra-deploy.generated.json`<br>`deploy/oss/helm/nvbes/Chart.yaml`<br>`deploy/oss/kustomize/kustomization.yaml`<br>`infrastructure/environments/staging/main.tf`<br>`infrastructure/environments/production/alloy.config.alloy`<br>`scripts/release-gate.sh`<br>`scripts/migrate-staging.sh` | `pnpm check:migration-infra-deploy && tofu -chdir=infrastructure/environments/development validate && tofu -chdir=infrastructure/environments/staging validate && pnpm check:supply-chain && pnpm check:migration-backup-restore && pnpm check:migration-observability` |
| P11 Repetitions migration | pending | no-go | migration lead required | `docs/migration/cutover-evidence-packet.generated.json`<br>`docs/migration/live-evidence-instances.generated.json`<br>`docs/migration/rehearsal-ledger.md`<br>`docs/migration/reconciliation.template.json`<br>`docs/migration/rejects.md` | `pnpm check:migration-rehearsals -- --strict && node tools/migration/reconcile.mjs --env staging --report docs/migration/reconciliation.<run>.json && pnpm check:migration-rejects -- --strict` |
| P12 Big Bang cutover | pending | no-go | migration lead required | `docs/migration/cutover-evidence-packet.generated.json`<br>`docs/migration/live-evidence-instances.generated.json`<br>`docs/migration/cutover-journal.md`<br>`docs/migration/cutover-checklist.md`<br>`docs/migration/observability-readiness.md`<br>`docs/migration/smoke-test-manifest.md` | `pnpm check:migration-precutover -- --env production --reconciliation-report docs/migration/reconciliation.<run>.json` |
| P13 Decommission | pending | no-go | migration lead required | `docs/migration/cutover-evidence-packet.generated.json`<br>`docs/migration/live-evidence-instances.generated.json`<br>`docs/migration/decommission-manifest.md`<br>`docs/migration/post-migration-audit.md` | `pnpm check:migration-postcutover` |

## Rules

- Phase rows must match the blueprint phase keys and titles.
- Summary counters must match the generated phase rows.
- Every `passed` or `accepted` phase evidence path must still exist in the repository.
- Generated Markdown must expose each phase owner, status, decision, evidence and proof.
- Pending cutover phases must expose the live evidence packet and artifacts required to unlock them.
- Generation provenance must identify source, write command and strict cutover command.

## Regeneration

```bash
pnpm check:migration-phase-ledger
tools/migration/phase-ledger.mjs --write
```
