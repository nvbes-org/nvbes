# Migration Domain Ledger

## Status

- domains: 8
- pending: 0
- accepted: 0
- passed: 8
- failed: 0

## Domains

| Domain | Status | Decision | Owner | Implementation Evidence | Migration Evidence | Proof |
|---|---:|---:|---|---|---|---|
| Identity | passed | go | Identity lead | `apps/identity-api/src/identity.domains.auth.routes.register.rs`<br>`apps/identity-api/src/identity.http.middleware.jwt.session_refresh.rs`<br>`apps/identity-api/src/identity.domains.auth.sessions.mgmt.rs`<br>`apps/identity-api/src/identity.domains.auth.routes.login.mfa_flow.rs`<br>`apps/identity-api/openapi.json` | `docs/migration/identity-register.generated.json`<br>`docs/migration/identity-login-session.generated.json`<br>`docs/migration/identity-mfa-webauthn.generated.json` | `pnpm check:migration-identity-register && pnpm check:migration-identity-login-session && pnpm check:migration-identity-mfa-webauthn` |
| Workspace | passed | go | Workspace lead | `libs/rust/core/src/authz.policy.rs`<br>`libs/rust/core/src/authz.policy.tests.rs`<br>`apps/identity-api/src/identity.domains.authz.service.rs`<br>`apps/identity-api/src/identity.domains.authz.db.rs` | `docs/migration/workspace-membership-roles.generated.json`<br>`docs/migration/workspace-last-owner.generated.json` | `pnpm check:migration-workspace-membership-roles && pnpm check:migration-workspace-last-owner` |
| Drive | passed | go | Drive lead | `apps/drive-api/src/drive.domains.uploads.core.rs`<br>`apps/drive-api/src/drive.domains.files.transfer.range.rs`<br>`apps/drive-api/src/drive.domains.share_links.logic.tests.rs`<br>`apps/drive-api/src/drive.domains.quotas.logic.rs`<br>`apps/drive-worker/src/drive.workers.maintenance.rs`<br>`apps/drive-api/openapi.json` | `docs/migration/drive-upload-download.generated.json`<br>`docs/migration/drive-share-revoke.generated.json`<br>`docs/migration/drive-quotas.generated.json` | `pnpm check:migration-drive-upload-download && pnpm check:migration-drive-share-revoke && pnpm check:migration-drive-quotas` |
| Billing | passed | go | Billing lead | `libs/rust/billing/src/views.rs`<br>`libs/rust/billing/src/types.rs`<br>`apps/identity-api/src/identity.domains.billing.webhooks.logic.rs`<br>`apps/drive-api/src/drive.domains.billing.manage.checkout.rs` | `docs/migration/billing-entitlements.generated.json`<br>`docs/migration/billing-webhook-idempotency.generated.json` | `pnpm check:migration-billing-entitlements && pnpm check:migration-billing-webhook-idempotency` |
| Audit | passed | go | Audit lead | `libs/rust/audit/src/lib.rs`<br>`apps/identity-api/src/identity.domains.auth.sessions.mgmt.rs`<br>`apps/drive-api/src/drive.domains.share_links.observability.rs` | `docs/migration/audit-append-only.generated.json` | `pnpm check:migration-audit-append-only` |
| Privacy | passed | go | Privacy lead | `apps/identity-worker/src/identity.worker.jobs.process_data_export.rs`<br>`apps/identity-api/src/identity.domains.auth.sessions.mgmt.rs`<br>`apps/identity-api/src/identity.email.jobs.rs` | `docs/migration/privacy-export-delete.generated.json` | `pnpm check:migration-privacy-export-delete` |
| Developer Platform | passed | go | Developer Platform lead | `apps/identity-api/src/identity.domains.developer.routes.oauth.rs`<br>`apps/identity-api/src/identity.domains.developer.routes.tokens.rs`<br>`apps/identity-api/src/identity.domains.developer.routes.webhooks.rs`<br>`apps/developer-web/src/developer.router.tsx`<br>`libs/ts/identity-sdk-core/openapi.json` | `docs/migration/developer-oauth-tokens.generated.json`<br>`docs/migration/developer-signed-webhooks.generated.json` | `pnpm check:migration-developer-oauth-tokens && pnpm check:migration-developer-signed-webhooks` |
| Cloud | passed | go | Cloud lead | `docs/migration/cloud-provisioning.generated.json`<br>`libs/go/provisioning/provisioning.go`<br>`libs/go/provisioning/provisioning_test.go`<br>`libs/go/control-plane/foundation.go`<br>`apps/cloud-control-api/README.md`<br>`apps/cloud-console/README.md` | `docs/migration/cloud-provisioning.generated.json`<br>`docs/migration/resource-map.generated.json`<br>`docs/migration/secret-map.generated.json` | `pnpm check:migration-cloud-provisioning && pnpm check:go && pnpm check:oss-boundaries` |

## Rules

- Domain rows must match the blueprint domain names and scopes.
- Summary counters must match generated domain rows.
- Every `passed` or `accepted` domain evidence path must still exist in the repository.
- Generated Markdown must expose implementation evidence, migration evidence and proof for every domain.
- Generation provenance must identify source, write command and strict cutover command.

## Regeneration

```bash
pnpm check:migration-domain-ledger
tools/migration/domain-ledger.mjs --write
```
