# Resource Migration Map

## Status

- entries: 43
- pending: 0
- keep: 24
- rebuild: 17
- remove: 0
- replace: 2

## Rules

- Every resource must have a target, owner, cutover handling and verification checks.
- Source rows, domains and summary counters must match the inventory-derived map.
- Generation provenance must identify source, write command and strict cutover command.

## Resources

| Type | Name | Decision | Owner | Target |
|---|---|---:|---|---|
| bucket | scaleway_object_bucket.archive | replace | Data lead | `deploy/oss/opentofu#s3-compatible-files-bucket` |
| bucket | scaleway_object_bucket.files | replace | Data lead | `deploy/oss/opentofu#s3-compatible-files-bucket` |
| event_topic | account.billing.facade.requested | keep | Platform lead | `contracts/events/account.billing.facade.requested.v1.schema.json` |
| event_topic | account.session.created | keep | Platform lead | `contracts/events/account.session.created.v1.schema.json` |
| event_topic | account.token.revoked | keep | Platform lead | `contracts/events/account.token.revoked.v1.schema.json` |
| event_topic | account.user.created | keep | Platform lead | `contracts/events/account.user.created.v1.schema.json` |
| event_topic | billing.dunning.changed | keep | Platform lead | `contracts/events/billing.dunning.changed.v1.schema.json` |
| event_topic | billing.entitlement.changed | keep | Platform lead | `contracts/events/billing.entitlement.changed.v1.schema.json` |
| event_topic | billing.invoice.issued | keep | Platform lead | `contracts/events/billing.invoice.issued.v1.schema.json` |
| event_topic | billing.payment.changed | keep | Platform lead | `contracts/events/billing.payment.changed.v1.schema.json` |
| event_topic | billing.reconciliation.difference | keep | Platform lead | `contracts/events/billing.reconciliation.difference.v1.schema.json` |
| event_topic | billing.subscription.changed | keep | Platform lead | `contracts/events/billing.subscription.changed.v1.schema.json` |
| event_topic | billing.usage.recorded | keep | Platform lead | `contracts/events/billing.usage.recorded.v1.schema.json` |
| event_topic | cloud.workspace.created | keep | Platform lead | `contracts/events/cloud.workspace.created.v1.schema.json` |
| event_topic | cloud.workspace.membership.created | keep | Platform lead | `contracts/events/cloud.workspace.membership.created.v1.schema.json` |
| event_topic | cloud.workspace.quota.changed | keep | Platform lead | `contracts/events/cloud.workspace.quota.changed.v1.schema.json` |
| event_topic | developer.app.created | keep | Platform lead | `contracts/events/developer.app.created.v1.schema.json` |
| event_topic | developer.token.created | keep | Platform lead | `contracts/events/developer.token.created.v1.schema.json` |
| event_topic | developer.webhook.endpoint.created | keep | Platform lead | `contracts/events/developer.webhook.endpoint.created.v1.schema.json` |
| event_topic | drive.file.created | keep | Platform lead | `contracts/events/drive.file.created.v1.schema.json` |
| event_topic | enterprise.access_review.started | keep | Platform lead | `contracts/events/enterprise.access_review.started.v1.schema.json` |
| event_topic | enterprise.break_glass.activated | keep | Platform lead | `contracts/events/enterprise.break_glass.activated.v1.schema.json` |
| event_topic | enterprise.federation.provider.changed | keep | Platform lead | `contracts/events/enterprise.federation.provider.changed.v1.schema.json` |
| event_topic | enterprise.policy.changed | keep | Platform lead | `contracts/events/enterprise.policy.changed.v1.schema.json` |
| event_topic | identity.user.created | keep | Platform lead | `contracts/events/identity.user.created.v1.schema.json` |
| event_topic | workspace.membership.created | keep | Platform lead | `contracts/events/workspace.membership.created.v1.schema.json` |
| queue | account.data_export | rebuild | Infra lead | `apps/workers#account-data-export` |
| queue | billing.email.send | rebuild | Infra lead | `apps/workers#billing-email-send` |
| queue | billing.mollie.webhook.process | rebuild | Infra lead | `apps/workers#billing-mollie-webhook-process` |
| queue | billing.stripe.webhook.process | rebuild | Infra lead | `apps/workers#billing-stripe-webhook-process` |
| queue | email.send | rebuild | Infra lead | `apps/workers#email-send` |
| queue | email.webhook.process | rebuild | Infra lead | `apps/workers#email-webhook-process` |
| queue | geo.lookup_maintenance | rebuild | Infra lead | `apps/workers#geo-lookup-maintenance` |
| queue | geo.loyalsoldier_import | rebuild | Infra lead | `apps/workers#geo-loyalsoldier-import` |
| queue | geo.maxmind_geolite_import | rebuild | Infra lead | `apps/workers#geo-maxmind-geolite-import` |
| queue | geo.v2fly_import | rebuild | Infra lead | `apps/workers#geo-v2fly-import` |
| queue | privacy.workspace_delete | rebuild | Infra lead | `apps/workers#privacy-workspace-delete` |
| queue | privacy.workspace_export | rebuild | Infra lead | `apps/workers#privacy-workspace-export` |
| queue | quotas.recalculate | rebuild | Infra lead | `apps/workers#quotas-recalculate` |
| queue | storage.purge_deleted | rebuild | Infra lead | `apps/workers#storage-purge-deleted` |
| queue | storage.purge_quarantined | rebuild | Infra lead | `apps/workers#storage-purge-quarantined` |
| queue | trash.purge | rebuild | Infra lead | `apps/workers#trash-purge` |
| queue | uploads.purge_expired | rebuild | Infra lead | `apps/workers#uploads-purge-expired` |

## Regeneration

```bash
pnpm check:migration-resource-map
node tools/migration/resource-map.mjs --write
```
