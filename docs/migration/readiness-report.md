# Migration Readiness Report

## Status

- production_cutover: no-go
- blocking_areas: 16
- blocking_items: 140
- live_evidence_missing_requirements: 6
- live_evidence_missing_items: 24

## Rules

- `production_cutover: go` requires zero blocking source rows.
- `blocking_areas` must match the generated blocker list.
- `blocking_items` must equal the sum of all blocker counts.
- Every blocker must reference a source row with the same blocking count.
- Every source and blocker must carry a proof command that references an existing package script or migration tool.
- Source rows must be unique, complete, path-matched and use non-negative counters.
- Generation provenance must identify write and strict cutover commands.

## Sources

| Area | Source | Total | Blocking | Proof |
|---|---|---:|---:|---|
| data | `docs/migration/data-map.generated.json` | 237 | 0 | `pnpm check:migration-data-map` |
| secrets | `docs/migration/secret-map.generated.json` | 249 | 0 | `pnpm check:migration-secret-map` |
| jobs | `docs/migration/job-map.generated.json` | 16 | 0 | `pnpm check:migration-job-map` |
| resources | `docs/migration/resource-map.generated.json` | 43 | 0 | `pnpm check:migration-resource-map` |
| target_structure | `docs/migration/target-structure.generated.json` | 56 | 5 | `pnpm check:migration-target-structure` |
| codegen | `docs/migration/codegen.generated.json` | 6 | 0 | `pnpm check:migration-codegen` |
| supply_chain | `docs/migration/supply-chain.generated.json` | 5 | 0 | `pnpm check:migration-supply-chain` |
| runtimes | `docs/migration/runtime-foundation.generated.json` | 4 | 0 | `pnpm check:migration-runtime-foundation` |
| platform_primitives | `docs/migration/platform-primitives.generated.json` | 9 | 0 | `pnpm check:migration-platform-primitives` |
| identity_register | `docs/migration/identity-register.generated.json` | 26 | 0 | `pnpm check:migration-identity-register` |
| identity_login_session | `docs/migration/identity-login-session.generated.json` | 23 | 0 | `pnpm check:migration-identity-login-session` |
| identity_mfa_webauthn | `docs/migration/identity-mfa-webauthn.generated.json` | 36 | 0 | `pnpm check:migration-identity-mfa-webauthn` |
| workspace_membership_roles | `docs/migration/workspace-membership-roles.generated.json` | 26 | 0 | `pnpm check:migration-workspace-membership-roles` |
| workspace_last_owner | `docs/migration/workspace-last-owner.generated.json` | 17 | 0 | `pnpm check:migration-workspace-last-owner` |
| drive_upload_download | `docs/migration/drive-upload-download.generated.json` | 48 | 0 | `pnpm check:migration-drive-upload-download` |
| drive_share_revoke | `docs/migration/drive-share-revoke.generated.json` | 32 | 0 | `pnpm check:migration-drive-share-revoke` |
| drive_quotas | `docs/migration/drive-quotas.generated.json` | 37 | 0 | `pnpm check:migration-drive-quotas` |
| audit_append_only | `docs/migration/audit-append-only.generated.json` | 34 | 0 | `pnpm check:migration-audit-append-only` |
| privacy_export_delete | `docs/migration/privacy-export-delete.generated.json` | 11 | 0 | `pnpm check:migration-privacy-export-delete` |
| billing_entitlements | `docs/migration/billing-entitlements.generated.json` | 36 | 0 | `pnpm check:migration-billing-entitlements` |
| billing_webhook_idempotency | `docs/migration/billing-webhook-idempotency.generated.json` | 46 | 0 | `pnpm check:migration-billing-webhook-idempotency` |
| billing_multi_psp_continuity | `docs/migration/billing-multi-psp-continuity.generated.json` | 20 | 0 | `pnpm check:migration-billing-multi-psp-continuity` |
| developer_oauth_tokens | `docs/migration/developer-oauth-tokens.generated.json` | 30 | 0 | `pnpm check:migration-developer-oauth-tokens` |
| developer_signed_webhooks | `docs/migration/developer-signed-webhooks.generated.json` | 27 | 0 | `pnpm check:migration-developer-signed-webhooks` |
| cloud_provisioning | `docs/migration/cloud-provisioning.generated.json` | 21 | 0 | `pnpm check:migration-cloud-provisioning` |
| phases | `docs/migration/phase-ledger.generated.json` | 13 | 3 | `node tools/migration/phase-ledger.mjs --strict` |
| domains | `docs/migration/domain-ledger.generated.json` | 8 | 0 | `pnpm check:migration-domain-ledger` |
| domain_dod | `docs/migration/domain-dod.generated.json` | 64 | 0 | `pnpm check:migration-domain-dod` |
| risks | `docs/migration/risk-register.generated.json` | 7 | 1 | `node tools/migration/risk-register.mjs --strict` |
| gates | `docs/migration/gate-evidence.generated.json` | 9 | 5 | `node tools/migration/gate-evidence.mjs --strict` |
| owner_signoffs | `docs/migration/owner-signoff-matrix.md` | 31 | 31 | `pnpm check:migration-owner-signoffs -- --strict` |
| cutover_checklist | `docs/migration/cutover-checklist.md` | 18 | 18 | `pnpm check:migration-cutover-checklist -- --strict` |
| rehearsals | `docs/migration/rehearsal-ledger.md` | 8 | 8 | `pnpm check:migration-rehearsals -- --strict` |
| snapshots | `docs/migration/snapshot-manifest.md` | 3 | 3 | `pnpm check:migration-snapshots -- --strict` |
| release_freeze | `docs/migration/release-freeze-manifest.md` | 13 | 13 | `pnpm check:migration-release-freeze -- --strict` |
| communication | `docs/migration/communication-plan.md` | 15 | 15 | `pnpm check:migration-communication -- --strict` |
| observability | `docs/migration/observability-readiness.md` | 13 | 13 | `pnpm check:migration-observability -- --strict` |
| rejects | `docs/migration/rejects.md` | 1 | 1 | `pnpm check:migration-rejects -- --strict` |
| reconciliation_report | `docs/migration/reconciliation-report.md` | 12 | 12 | `pnpm check:migration-reconciliation-report -- --strict` |
| cutover_journal | `docs/migration/cutover-journal.md` | 5 | 5 | `pnpm check:migration-cutover-journal -- --strict` |
| reconciliation_template | `docs/migration/reconciliation.template.json` | 1 | 1 | `node tools/migration/reconcile.mjs --env production --report docs/migration/reconciliation.<run>.json` |
| live_evidence | `docs/migration/live-evidence-instances.generated.json` | 6 | 6 | `node tools/migration/live-evidence-instances.mjs --strict` |

## Blockers

- target_structure: 5 blocking item(s) in `docs/migration/target-structure.generated.json`; proof: `pnpm check:migration-target-structure`
- phases: 3 blocking item(s) in `docs/migration/phase-ledger.generated.json`; proof: `node tools/migration/phase-ledger.mjs --strict`
- risks: 1 blocking item(s) in `docs/migration/risk-register.generated.json`; proof: `node tools/migration/risk-register.mjs --strict`
- gates: 5 blocking item(s) in `docs/migration/gate-evidence.generated.json`; proof: `node tools/migration/gate-evidence.mjs --strict`
- owner_signoffs: 31 blocking item(s) in `docs/migration/owner-signoff-matrix.md`; proof: `pnpm check:migration-owner-signoffs -- --strict`
- cutover_checklist: 18 blocking item(s) in `docs/migration/cutover-checklist.md`; proof: `pnpm check:migration-cutover-checklist -- --strict`
- rehearsals: 8 blocking item(s) in `docs/migration/rehearsal-ledger.md`; proof: `pnpm check:migration-rehearsals -- --strict`
- snapshots: 3 blocking item(s) in `docs/migration/snapshot-manifest.md`; proof: `pnpm check:migration-snapshots -- --strict`
- release_freeze: 13 blocking item(s) in `docs/migration/release-freeze-manifest.md`; proof: `pnpm check:migration-release-freeze -- --strict`
- communication: 15 blocking item(s) in `docs/migration/communication-plan.md`; proof: `pnpm check:migration-communication -- --strict`
- observability: 13 blocking item(s) in `docs/migration/observability-readiness.md`; proof: `pnpm check:migration-observability -- --strict`
- rejects: 1 blocking item(s) in `docs/migration/rejects.md`; proof: `pnpm check:migration-rejects -- --strict`
- reconciliation_report: 12 blocking item(s) in `docs/migration/reconciliation-report.md`; proof: `pnpm check:migration-reconciliation-report -- --strict`
- cutover_journal: 5 blocking item(s) in `docs/migration/cutover-journal.md`; proof: `pnpm check:migration-cutover-journal -- --strict`
- reconciliation_template: 1 blocking item(s) in `docs/migration/reconciliation.template.json`; proof: `node tools/migration/reconcile.mjs --env production --report docs/migration/reconciliation.<run>.json`
- live_evidence: 6 blocking item(s) in `docs/migration/live-evidence-instances.generated.json`; proof: `node tools/migration/live-evidence-instances.mjs --strict`

## Regeneration

```bash
pnpm check:migration-readiness-report
node tools/migration/readiness-report.mjs --write
```
