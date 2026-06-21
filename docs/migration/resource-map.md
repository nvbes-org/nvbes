# Resource Migration Map

## Status

- entries: 18
- pending: 0
- keep: 4
- rebuild: 13
- remove: 0
- replace: 1

## Rules

- Every resource must have a target, owner, cutover handling and verification checks.
- Source rows, domains and summary counters must match the inventory-derived map.
- Generation provenance must identify source, write command and strict cutover command.

## Resources

| Type | Name | Decision | Owner | Target |
|---|---|---:|---|---|
| bucket | scaleway_object_bucket.files | replace | Data lead | `deploy/oss/opentofu#s3-compatible-files-bucket` |
| event_topic | billing.entitlement.changed | keep | Platform lead | `contracts/events/billing.entitlement.changed.v1.schema.json` |
| event_topic | drive.file.created | keep | Platform lead | `contracts/events/drive.file.created.v1.schema.json` |
| event_topic | identity.user.created | keep | Platform lead | `contracts/events/identity.user.created.v1.schema.json` |
| event_topic | workspace.membership.created | keep | Platform lead | `contracts/events/workspace.membership.created.v1.schema.json` |
| queue | billing.stripe.webhook.process | rebuild | Infra lead | `apps/workers#billing-usage-billing-stripe-webhook-process` |
| queue | data.export | rebuild | Infra lead | `apps/workers#audit-privacy-data-export` |
| queue | email.send | rebuild | Infra lead | `apps/workers#email-email-send` |
| queue | email.webhook.process | rebuild | Infra lead | `apps/workers#email-email-webhook-process` |
| queue | privacy.account_delete | rebuild | Infra lead | `apps/workers#audit-privacy-privacy-account-delete` |
| queue | privacy.account_export | rebuild | Infra lead | `apps/workers#audit-privacy-privacy-account-export` |
| queue | privacy.workspace_delete | rebuild | Infra lead | `apps/workers#audit-privacy-privacy-workspace-delete` |
| queue | privacy.workspace_export | rebuild | Infra lead | `apps/workers#audit-privacy-privacy-workspace-export` |
| queue | quotas.recalculate | rebuild | Infra lead | `apps/workers#drive-quotas-recalculate` |
| queue | storage.purge_deleted | rebuild | Infra lead | `apps/workers#drive-storage-purge-deleted` |
| queue | storage.purge_quarantined | rebuild | Infra lead | `apps/workers#drive-storage-purge-quarantined` |
| queue | trash.purge | rebuild | Infra lead | `apps/workers#drive-trash-purge` |
| queue | uploads.purge_expired | rebuild | Infra lead | `apps/workers#drive-uploads-purge-expired` |

## Regeneration

```bash
pnpm check:migration-resource-map
tools/migration/resource-map.mjs --write
```
