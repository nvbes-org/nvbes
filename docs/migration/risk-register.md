# Migration Risk Register

## Status

- Not signed for cutover while any risk remains `pending`.

- risks: 7
- pending: 1
- mitigated: 6
- accepted: 0
- removed: 0

## Risks

| Risk | Owner | Severity | Status | Evidence | Cutover impact |
|---|---|---|---|---|---|
| modele data legacy ambigu | Data lead | blocking | mitigated | `docs/migration/data-migration-pipeline.generated.json`<br>`docs/migration/data-map.generated.json`<br>`docs/migration/rejects.md`<br>pnpm check:migration-data-migration-pipeline | allowed for repository gates: every source table has a target, owner, reconciliation checks and reject rule; production still needs an executed reconciliation report |
| sessions incompatibles | Product lead | blocking | mitigated | `apps/identity-web/src/identity.return-to.ts`<br>`apps/identity-web/src/identity.return-to.test.js`<br>`apps/developer-web/src/developer.session.ts`<br>`apps/developer-web/src/__tests__/developer.session.test.ts` | allowed: incompatible sessions are handled by explicit logout/session clearing and a validated login return flow |
| objets storage manquants | Drive lead | blocking | mitigated | `docs/migration/data-migration-pipeline.generated.json`<br>`docs/migration/drive-upload-download.generated.json`<br>`docs/migration/drive-share-revoke.generated.json` | allowed for repository gates: object/link invariants are encoded in the data map and Drive evidence; production still needs object-store reconciliation counts |
| divergence billing | Billing lead | blocking | mitigated | `docs/migration/data-migration-pipeline.generated.json`<br>`docs/migration/billing-entitlements.generated.json`<br>`docs/migration/billing-webhook-idempotency.generated.json`<br>`docs/migration/billing-multi-psp-continuity.generated.json`<br>`docs/migration/release-freeze-manifest.md` | allowed for repository gates: ledger-balance reconciliation, multi-PSP continuity and billing mutation freeze are documented; production still needs accepted ledger reconciliation |
| event replay non idempotent | Infra lead | blocking | mitigated | `docs/migration/job-map.generated.json`<br>`docs/migration/billing-webhook-idempotency.generated.json`<br>`docs/migration/developer-signed-webhooks.generated.json`<br>pnpm check:migration-job-map | allowed for repository gates: critical jobs and webhook replay paths carry idempotency-key evidence; production still needs DLQ and lag evidence in rehearsals |
| rollback lent | infra lead required | blocking | pending | `docs/migration/rollback-report.md`<br>`docs/migration/rehearsal-ledger.md`<br>`docs/migration/live-evidence-instances.generated.json`<br>pnpm check:migration-rollback-report -- --strict | no-go until an infra owner attaches accepted rollback live evidence with timed rollback proof from staging rehearsal |
| fuite Cloud/Internal dans OSS | Security lead | blocking | mitigated | `tools/oss-export/checks.mjs`<br>`tools/oss-export/manifest.json`<br>`scripts/check-oss-provider-boundaries.mjs`<br>pnpm check:oss-boundaries | allowed: OSS export allowlist and provider boundary checks block private docs and Cloud/Internal leakage |

## Rules

- Every blocking risk from the blueprint must have an owner.
- Every blocking risk must have mitigation evidence before cutover.
- Every `mitigated` or `accepted` risk file evidence path must still exist in the repository.
- A risk can pass cutover only as `mitigated`, `accepted` by owner, or `removed` from scope.
- `pending` risks are no-go.
- Pending risks must expose the evidence required to clear the no-go state.
- Generated Markdown must expose each risk owner, severity, status, evidence and cutover impact.
- Generation provenance must identify the blueprint source, write command and strict cutover command.

## Review Rules

| Status | Meaning | Cutover impact |
|---|---|---|
| pending | owner or evidence missing | no-go |
| mitigated | mitigation implemented and evidenced | allowed |
| accepted | owner accepts residual risk | allowed with signed impact |
| removed | risk source removed from scope | allowed |

## Regeneration

```bash
tools/migration/risk-register.mjs --write
pnpm check:migration-risk-register
```

Before a production cutover, run the strict gate:

```bash
tools/migration/risk-register.mjs --strict
```
