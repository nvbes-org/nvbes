# Migration Readiness Report

## Status

- production_cutover: no-go
- blocking_areas: 6
- blocking_items: 21
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
| data | `docs/migration/data-map.generated.json` | 179 | 0 | `pnpm check:migration-data-map` |
| secrets | `docs/migration/secret-map.generated.json` | 80 | 0 | `pnpm check:migration-secret-map` |
| jobs | `docs/migration/job-map.generated.json` | 14 | 0 | `pnpm check:migration-job-map` |
| resources | `docs/migration/resource-map.generated.json` | 25 | 0 | `pnpm check:migration-resource-map` |
| target_structure | `docs/migration/target-structure.generated.json` | 56 | 0 | `pnpm check:migration-target-structure` |
| codegen | `docs/migration/codegen.generated.json` | 6 | 0 | `pnpm check:migration-codegen` |
| supply_chain | `docs/migration/supply-chain.generated.json` | 5 | 0 | `pnpm check:migration-supply-chain` |
| runtimes | `docs/migration/runtime-foundation.generated.json` | 4 | 0 | `pnpm check:migration-runtime-foundation` |
| platform_primitives | `docs/migration/platform-primitives.generated.json` | 9 | 0 | `pnpm check:migration-platform-primitives` |
| identity_register | `docs/migration/identity-register.generated.json` | 29 | 0 | `pnpm check:migration-identity-register` |
| identity_login_session | `docs/migration/identity-login-session.generated.json` | 27 | 0 | `pnpm check:migration-identity-login-session` |
| identity_mfa_webauthn | `docs/migration/identity-mfa-webauthn.generated.json` | 37 | 0 | `pnpm check:migration-identity-mfa-webauthn` |
| workspace_membership_roles | `docs/migration/workspace-membership-roles.generated.json` | 26 | 0 | `pnpm check:migration-workspace-membership-roles` |
| workspace_last_owner | `docs/migration/workspace-last-owner.generated.json` | 18 | 0 | `pnpm check:migration-workspace-last-owner` |
| drive_upload_download | `docs/migration/drive-upload-download.generated.json` | 48 | 0 | `pnpm check:migration-drive-upload-download` |
| drive_share_revoke | `docs/migration/drive-share-revoke.generated.json` | 32 | 0 | `pnpm check:migration-drive-share-revoke` |
| drive_quotas | `docs/migration/drive-quotas.generated.json` | 37 | 0 | `pnpm check:migration-drive-quotas` |
| audit_append_only | `docs/migration/audit-append-only.generated.json` | 34 | 0 | `pnpm check:migration-audit-append-only` |
| privacy_export_delete | `docs/migration/privacy-export-delete.generated.json` | 29 | 0 | `pnpm check:migration-privacy-export-delete` |
| billing_entitlements | `docs/migration/billing-entitlements.generated.json` | 35 | 0 | `pnpm check:migration-billing-entitlements` |
| billing_webhook_idempotency | `docs/migration/billing-webhook-idempotency.generated.json` | 15 | 0 | `pnpm check:migration-billing-webhook-idempotency` |
| developer_oauth_tokens | `docs/migration/developer-oauth-tokens.generated.json` | 30 | 0 | `pnpm check:migration-developer-oauth-tokens` |
| developer_signed_webhooks | `docs/migration/developer-signed-webhooks.generated.json` | 27 | 0 | `pnpm check:migration-developer-signed-webhooks` |
| cloud_provisioning | `docs/migration/cloud-provisioning.generated.json` | 21 | 0 | `pnpm check:migration-cloud-provisioning` |
| phases | `docs/migration/phase-ledger.generated.json` | 13 | 3 | `tools/migration/phase-ledger.mjs --strict` |
| domains | `docs/migration/domain-ledger.generated.json` | 8 | 0 | `pnpm check:migration-domain-ledger` |
| domain_dod | `docs/migration/domain-dod.generated.json` | 64 | 0 | `pnpm check:migration-domain-dod` |
| risks | `docs/migration/risk-register.generated.json` | 7 | 1 | `tools/migration/risk-register.mjs --strict` |
| gates | `docs/migration/gate-evidence.generated.json` | 9 | 5 | `tools/migration/gate-evidence.mjs --strict` |
| gate_decisions | `docs/migration/gate-evidence.generated.json` | 9 | 5 | `tools/migration/gate-evidence.mjs --strict` |
| reconciliation_template | `docs/migration/reconciliation.template.json` | 1 | 1 | `tools/migration/reconcile.mjs --env production --report docs/migration/reconciliation.<run>.json` |
| live_evidence | `docs/migration/live-evidence-instances.generated.json` | 6 | 6 | `tools/migration/live-evidence-instances.mjs --strict` |

## Blockers

- phases: 3 blocking item(s) in `docs/migration/phase-ledger.generated.json`; proof: `tools/migration/phase-ledger.mjs --strict`
- risks: 1 blocking item(s) in `docs/migration/risk-register.generated.json`; proof: `tools/migration/risk-register.mjs --strict`
- gates: 5 blocking item(s) in `docs/migration/gate-evidence.generated.json`; proof: `tools/migration/gate-evidence.mjs --strict`
- gate_decisions: 5 blocking item(s) in `docs/migration/gate-evidence.generated.json`; proof: `tools/migration/gate-evidence.mjs --strict`
- reconciliation_template: 1 blocking item(s) in `docs/migration/reconciliation.template.json`; proof: `tools/migration/reconcile.mjs --env production --report docs/migration/reconciliation.<run>.json`
- live_evidence: 6 blocking item(s) in `docs/migration/live-evidence-instances.generated.json`; proof: `tools/migration/live-evidence-instances.mjs --strict`

## Regeneration

```bash
pnpm check:migration-readiness-report
tools/migration/readiness-report.mjs --write
```
