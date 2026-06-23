# Job Migration Map

## Status

- entries: 14
- pending: 0
- keep: 0
- rebuild: 14
- remove: 0
- replace: 0

## Rules

- Every job must have a target worker, replacement path, owner and verification checks.
- Source rows, domains and summary counters must match the inventory-derived map.
- Generation provenance must identify source, write command and strict cutover command.

## Jobs

| Job | Domain | Decision | Owner | Target | Replacement |
|---|---|---:|---|---|---|
| geo.lookup_maintenance | Platform | rebuild | Infra lead | `apps/workers#geo-lookup-maintenance` | `target-worker:geo.lookup_maintenance` |
| quotas.recalculate | Drive | rebuild | Drive lead | `apps/workers#quotas-recalculate` | `target-worker:quotas.recalculate` |
| storage.purge_deleted | Drive | rebuild | Drive lead | `apps/workers#storage-purge-deleted` | `target-worker:storage.purge_deleted` |
| storage.purge_quarantined | Drive | rebuild | Drive lead | `apps/workers#storage-purge-quarantined` | `target-worker:storage.purge_quarantined` |
| trash.purge | Drive | rebuild | Drive lead | `apps/workers#trash-purge` | `target-worker:trash.purge` |
| uploads.purge_expired | Drive | rebuild | Drive lead | `apps/workers#uploads-purge-expired` | `target-worker:uploads.purge_expired` |
| privacy.account_delete | Audit/Privacy | rebuild | Privacy lead | `apps/workers#privacy-account-delete` | `target-worker:privacy.account_delete` |
| privacy.workspace_delete | Audit/Privacy | rebuild | Privacy lead | `apps/workers#privacy-workspace-delete` | `target-worker:privacy.workspace_delete` |
| privacy.account_export | Audit/Privacy | rebuild | Privacy lead | `apps/workers#privacy-account-export` | `target-worker:privacy.account_export` |
| privacy.workspace_export | Audit/Privacy | rebuild | Privacy lead | `apps/workers#privacy-workspace-export` | `target-worker:privacy.workspace_export` |
| billing.stripe.webhook.process | Billing/Usage | rebuild | Billing lead | `apps/workers#billing-stripe-webhook-process` | `target-worker:billing.stripe.webhook.process` |
| data.export | Audit/Privacy | rebuild | Privacy lead | `apps/workers#data-export` | `target-worker:data.export` |
| email.send | Email | rebuild | Platform lead | `apps/workers#email-send` | `target-worker:email.send` |
| email.webhook.process | Email | rebuild | Platform lead | `apps/workers#email-webhook-process` | `target-worker:email.webhook.process` |

## Regeneration

```bash
pnpm check:migration-job-map
tools/migration/job-map.mjs --write
```
