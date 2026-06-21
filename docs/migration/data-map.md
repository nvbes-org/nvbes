# Data Migration Map

## Status

- entries: 104
- pending: 0
- keep: 0
- rebuild: 104
- remove: 0
- replace: 0

## Rules

- Every source table maps to a target table, reject rule, or deletion rule.
- Critical data requires row counts and checksums; financial data also requires ledger balance.
- Personal data carries classification, retention, and owner decisions.
- Source rows, domains and summary counters must match the inventory-derived map.
- Production cutover still requires a separate reconciliation report with accepted checksums.
- Generation provenance must identify source, write command and strict cutover command.

## Table Map

| Source | Target | Domain | Classification | Retention | Reconciliation | Status |
|---|---|---|---|---|---|---|
| apps/drive-api/migrations/0001_initial_schema.sql#api_key_nonces | `target-postgres:drive.api_key_nonces` | Drive | sensitive personal data | shortest-legal-retention | row_count, checksum | rebuild |
| apps/drive-api/migrations/0001_initial_schema.sql#api_keys | `target-postgres:drive.api_keys` | Drive | sensitive personal data | shortest-legal-retention | row_count, checksum | rebuild |
| apps/drive-api/migrations/0001_initial_schema.sql#api_request_logs | `target-postgres:drive.api_request_logs` | Drive | operational data | operational-retention | row_count, checksum | rebuild |
| apps/drive-api/migrations/0001_initial_schema.sql#audit_events | `target-postgres:audit-privacy.audit_events` | Audit/Privacy | audit data | append-only-audit-retention | row_count, checksum | rebuild |
| apps/drive-api/migrations/0001_initial_schema.sql#billing_accounts | `target-postgres:billing-usage.billing_accounts` | Billing/Usage | financial data | contractual-financial-retention | row_count, checksum, ledger_balance | rebuild |
| apps/drive-api/migrations/0001_initial_schema.sql#billing_adjustments | `target-postgres:billing-usage.billing_adjustments` | Billing/Usage | financial data | contractual-financial-retention | row_count, checksum, ledger_balance | rebuild |
| apps/drive-api/migrations/0001_initial_schema.sql#billing_webhook_events | `target-postgres:billing-usage.billing_webhook_events` | Billing/Usage | financial data | contractual-financial-retention | row_count, checksum, ledger_balance | rebuild |
| apps/drive-api/migrations/0001_initial_schema.sql#email_verification_tokens | `target-postgres:drive.email_verification_tokens` | Drive | sensitive personal data | shortest-legal-retention | row_count | rebuild |
| apps/drive-api/migrations/0001_initial_schema.sql#invoice_estimates | `target-postgres:billing-usage.invoice_estimates` | Billing/Usage | financial data | contractual-financial-retention | row_count, checksum, ledger_balance | rebuild |
| apps/drive-api/migrations/0001_initial_schema.sql#mfa_factors | `target-postgres:drive.mfa_factors` | Drive | sensitive personal data | shortest-legal-retention | row_count, checksum | rebuild |
| apps/drive-api/migrations/0001_initial_schema.sql#organization_memberships | `target-postgres:workspace-authz.organization_memberships` | Workspace/Authz | personal data | product-lifecycle-plus-privacy-retention | row_count, checksum, orphan_check | rebuild |
| apps/drive-api/migrations/0001_initial_schema.sql#organizations | `target-postgres:workspace-authz.organizations` | Workspace/Authz | operational data | operational-retention | row_count, checksum | rebuild |
| apps/drive-api/migrations/0001_initial_schema.sql#password_reset_tokens | `target-postgres:drive.password_reset_tokens` | Drive | sensitive personal data | shortest-legal-retention | row_count | rebuild |
| apps/drive-api/migrations/0001_initial_schema.sql#plans | `target-postgres:billing-usage.plans` | Billing/Usage | operational data | operational-retention | row_count, checksum | rebuild |
| apps/drive-api/migrations/0001_initial_schema.sql#privacy_requests | `target-postgres:audit-privacy.privacy_requests` | Audit/Privacy | operational data | operational-retention | row_count, checksum | rebuild |
| apps/drive-api/migrations/0001_initial_schema.sql#quota_usage | `target-postgres:billing-usage.quota_usage` | Billing/Usage | financial data | contractual-financial-retention | row_count, checksum, ledger_balance | rebuild |
| apps/drive-api/migrations/0001_initial_schema.sql#sessions | `target-postgres:drive.sessions` | Drive | sensitive personal data | shortest-legal-retention | row_count, checksum | rebuild |
| apps/drive-api/migrations/0001_initial_schema.sql#share_links | `target-postgres:drive.share_links` | Drive | operational data | operational-retention | row_count, checksum, object_or_link_invariant | rebuild |
| apps/drive-api/migrations/0001_initial_schema.sql#storage_objects | `target-postgres:drive.storage_objects` | Drive | operational data | operational-retention | row_count, checksum, object_or_link_invariant | rebuild |
| apps/drive-api/migrations/0001_initial_schema.sql#stripe_price_mappings | `target-postgres:billing-usage.stripe_price_mappings` | Billing/Usage | operational data | operational-retention | row_count, checksum | rebuild |
| apps/drive-api/migrations/0001_initial_schema.sql#subscriptions | `target-postgres:billing-usage.subscriptions` | Billing/Usage | financial data | contractual-financial-retention | row_count, checksum, ledger_balance | rebuild |
| apps/drive-api/migrations/0001_initial_schema.sql#tenant_memberships | `target-postgres:workspace-authz.tenant_memberships` | Workspace/Authz | personal data | product-lifecycle-plus-privacy-retention | row_count, checksum, orphan_check | rebuild |
| apps/drive-api/migrations/0001_initial_schema.sql#tenants | `target-postgres:workspace-authz.tenants` | Workspace/Authz | personal data | product-lifecycle-plus-privacy-retention | row_count, checksum, orphan_check | rebuild |
| apps/drive-api/migrations/0001_initial_schema.sql#upload_parts | `target-postgres:drive.upload_parts` | Drive | operational data | operational-retention | row_count, checksum, object_or_link_invariant | rebuild |
| apps/drive-api/migrations/0001_initial_schema.sql#upload_sessions | `target-postgres:drive.upload_sessions` | Drive | sensitive personal data | shortest-legal-retention | row_count, checksum, object_or_link_invariant | rebuild |
| apps/drive-api/migrations/0001_initial_schema.sql#usage_events | `target-postgres:billing-usage.usage_events` | Billing/Usage | financial data | contractual-financial-retention | row_count, checksum, ledger_balance | rebuild |
| apps/drive-api/migrations/0001_initial_schema.sql#usage_snapshots | `target-postgres:billing-usage.usage_snapshots` | Billing/Usage | financial data | contractual-financial-retention | row_count, checksum, ledger_balance | rebuild |
| apps/drive-api/migrations/0001_initial_schema.sql#users | `target-postgres:drive.users` | Drive | personal data | product-lifecycle-plus-privacy-retention | row_count, checksum, orphan_check | rebuild |
| apps/drive-api/migrations/0001_initial_schema.sql#workspace_invitations | `target-postgres:workspace-authz.workspace_invitations` | Workspace/Authz | personal data | product-lifecycle-plus-privacy-retention | row_count, checksum, orphan_check | rebuild |
| apps/drive-api/migrations/0001_initial_schema.sql#workspace_members | `target-postgres:workspace-authz.workspace_members` | Workspace/Authz | personal data | product-lifecycle-plus-privacy-retention | row_count, checksum, orphan_check | rebuild |
| apps/drive-api/migrations/0001_initial_schema.sql#workspace_memberships | `target-postgres:workspace-authz.workspace_memberships` | Workspace/Authz | personal data | product-lifecycle-plus-privacy-retention | row_count, checksum, orphan_check | rebuild |
| apps/drive-api/migrations/0001_initial_schema.sql#workspace_policies | `target-postgres:workspace-authz.workspace_policies` | Workspace/Authz | personal data | product-lifecycle-plus-privacy-retention | row_count, checksum, orphan_check | rebuild |
| apps/drive-api/migrations/0001_initial_schema.sql#workspaces | `target-postgres:workspace-authz.workspaces` | Workspace/Authz | personal data | product-lifecycle-plus-privacy-retention | row_count, checksum, orphan_check | rebuild |
| apps/identity-api/migrations/0001_initial_schema.sql#audit_events | `target-postgres:audit-privacy.audit_events` | Audit/Privacy | audit data | append-only-audit-retention | row_count, checksum | rebuild |
| apps/identity-api/migrations/0001_initial_schema.sql#billing_accounts | `target-postgres:billing-usage.billing_accounts` | Billing/Usage | financial data | contractual-financial-retention | row_count, checksum, ledger_balance | rebuild |
| apps/identity-api/migrations/0001_initial_schema.sql#billing_webhook_events | `target-postgres:billing-usage.billing_webhook_events` | Billing/Usage | financial data | contractual-financial-retention | row_count, checksum, ledger_balance | rebuild |
| apps/identity-api/migrations/0001_initial_schema.sql#bots | `target-postgres:identity.bots` | Identity | operational data | operational-retention | row_count, checksum | rebuild |
| apps/identity-api/migrations/0001_initial_schema.sql#devices | `target-postgres:identity.devices` | Identity | operational data | operational-retention | row_count, checksum | rebuild |
| apps/identity-api/migrations/0001_initial_schema.sql#email_events | `target-postgres:identity.email_events` | Identity | personal data | product-lifecycle-plus-privacy-retention | row_count, checksum | rebuild |
| apps/identity-api/migrations/0001_initial_schema.sql#email_messages | `target-postgres:identity.email_messages` | Identity | personal data | product-lifecycle-plus-privacy-retention | row_count, checksum | rebuild |
| apps/identity-api/migrations/0001_initial_schema.sql#enterprise_password_recovery_requests | `target-postgres:identity.enterprise_password_recovery_requests` | Identity | sensitive personal data | shortest-legal-retention | row_count, checksum | rebuild |
| apps/identity-api/migrations/0001_initial_schema.sql#federated_identity_providers | `target-postgres:identity.federated_identity_providers` | Identity | operational data | operational-retention | row_count, checksum | rebuild |
| apps/identity-api/migrations/0001_initial_schema.sql#idempotency_responses | `target-postgres:identity.idempotency_responses` | Identity | operational data | operational-retention | row_count, checksum | rebuild |
| apps/identity-api/migrations/0001_initial_schema.sql#invoice_estimates | `target-postgres:billing-usage.invoice_estimates` | Billing/Usage | financial data | contractual-financial-retention | row_count, checksum, ledger_balance | rebuild |
| apps/identity-api/migrations/0001_initial_schema.sql#mfa_factors | `target-postgres:identity.mfa_factors` | Identity | sensitive personal data | shortest-legal-retention | row_count, checksum | rebuild |
| apps/identity-api/migrations/0001_initial_schema.sql#oauth_client_assertion_jtis | `target-postgres:developer-platform.oauth_client_assertion_jtis` | Developer Platform | operational data | operational-retention | row_count, checksum | rebuild |
| apps/identity-api/migrations/0001_initial_schema.sql#oauth_client_policies | `target-postgres:developer-platform.oauth_client_policies` | Developer Platform | operational data | operational-retention | row_count, checksum | rebuild |
| apps/identity-api/migrations/0001_initial_schema.sql#oauth_clients | `target-postgres:developer-platform.oauth_clients` | Developer Platform | operational data | operational-retention | row_count, checksum | rebuild |
| apps/identity-api/migrations/0001_initial_schema.sql#oauth_consents | `target-postgres:audit-privacy.oauth_consents` | Audit/Privacy | operational data | operational-retention | row_count, checksum | rebuild |
| apps/identity-api/migrations/0001_initial_schema.sql#oauth_scope_metadata | `target-postgres:identity.oauth_scope_metadata` | Identity | operational data | operational-retention | row_count, checksum | rebuild |
| apps/identity-api/migrations/0001_initial_schema.sql#organization_memberships | `target-postgres:workspace-authz.organization_memberships` | Workspace/Authz | personal data | product-lifecycle-plus-privacy-retention | row_count, checksum, orphan_check | rebuild |
| apps/identity-api/migrations/0001_initial_schema.sql#organizations | `target-postgres:workspace-authz.organizations` | Workspace/Authz | operational data | operational-retention | row_count, checksum | rebuild |
| apps/identity-api/migrations/0001_initial_schema.sql#password_history | `target-postgres:identity.password_history` | Identity | sensitive personal data | shortest-legal-retention | row_count, checksum | rebuild |
| apps/identity-api/migrations/0001_initial_schema.sql#plans | `target-postgres:billing-usage.plans` | Billing/Usage | operational data | operational-retention | row_count, checksum | rebuild |
| apps/identity-api/migrations/0001_initial_schema.sql#pow_challenges | `target-postgres:identity.pow_challenges` | Identity | operational data | operational-retention | row_count, checksum | rebuild |
| apps/identity-api/migrations/0001_initial_schema.sql#principals | `target-postgres:identity.principals` | Identity | operational data | operational-retention | row_count, checksum | rebuild |
| apps/identity-api/migrations/0001_initial_schema.sql#quota_usage | `target-postgres:billing-usage.quota_usage` | Billing/Usage | financial data | contractual-financial-retention | row_count, checksum, ledger_balance | rebuild |
| apps/identity-api/migrations/0001_initial_schema.sql#risk_events | `target-postgres:identity.risk_events` | Identity | audit data | append-only-audit-retention | row_count, checksum | rebuild |
| apps/identity-api/migrations/0001_initial_schema.sql#saml_assertion_ids | `target-postgres:identity.saml_assertion_ids` | Identity | operational data | operational-retention | row_count, checksum | rebuild |
| apps/identity-api/migrations/0001_initial_schema.sql#saml_pending_requests | `target-postgres:identity.saml_pending_requests` | Identity | operational data | operational-retention | row_count, checksum | rebuild |
| apps/identity-api/migrations/0001_initial_schema.sql#saml_sp_config | `target-postgres:identity.saml_sp_config` | Identity | operational data | operational-retention | row_count, checksum | rebuild |
| apps/identity-api/migrations/0001_initial_schema.sql#scim_provisioning_connectors | `target-postgres:identity.scim_provisioning_connectors` | Identity | operational data | operational-retention | row_count, checksum | rebuild |
| apps/identity-api/migrations/0001_initial_schema.sql#service_accounts | `target-postgres:identity.service_accounts` | Identity | operational data | operational-retention | row_count, checksum | rebuild |
| apps/identity-api/migrations/0001_initial_schema.sql#signing_keys | `target-postgres:identity.signing_keys` | Identity | sensitive personal data | shortest-legal-retention | row_count, checksum | rebuild |
| apps/identity-api/migrations/0001_initial_schema.sql#stripe_price_mappings | `target-postgres:billing-usage.stripe_price_mappings` | Billing/Usage | operational data | operational-retention | row_count, checksum | rebuild |
| apps/identity-api/migrations/0001_initial_schema.sql#subscriptions | `target-postgres:billing-usage.subscriptions` | Billing/Usage | financial data | contractual-financial-retention | row_count, checksum, ledger_balance | rebuild |
| apps/identity-api/migrations/0001_initial_schema.sql#suppressed_emails | `target-postgres:identity.suppressed_emails` | Identity | personal data | product-lifecycle-plus-privacy-retention | row_count, checksum | rebuild |
| apps/identity-api/migrations/0001_initial_schema.sql#tenant_domains | `target-postgres:workspace-authz.tenant_domains` | Workspace/Authz | personal data | product-lifecycle-plus-privacy-retention | row_count, checksum, orphan_check | rebuild |
| apps/identity-api/migrations/0001_initial_schema.sql#tenant_memberships | `target-postgres:workspace-authz.tenant_memberships` | Workspace/Authz | personal data | product-lifecycle-plus-privacy-retention | row_count, checksum, orphan_check | rebuild |
| apps/identity-api/migrations/0001_initial_schema.sql#tenants | `target-postgres:workspace-authz.tenants` | Workspace/Authz | personal data | product-lifecycle-plus-privacy-retention | row_count, checksum, orphan_check | rebuild |
| apps/identity-api/migrations/0001_initial_schema.sql#usage_events | `target-postgres:billing-usage.usage_events` | Billing/Usage | financial data | contractual-financial-retention | row_count, checksum, ledger_balance | rebuild |
| apps/identity-api/migrations/0001_initial_schema.sql#usage_snapshots | `target-postgres:billing-usage.usage_snapshots` | Billing/Usage | financial data | contractual-financial-retention | row_count, checksum, ledger_balance | rebuild |
| apps/identity-api/migrations/0001_initial_schema.sql#user_consents | `target-postgres:audit-privacy.user_consents` | Audit/Privacy | personal data | product-lifecycle-plus-privacy-retention | row_count, checksum, orphan_check | rebuild |
| apps/identity-api/migrations/0001_initial_schema.sql#user_identities | `target-postgres:identity.user_identities` | Identity | personal data | product-lifecycle-plus-privacy-retention | row_count, checksum, orphan_check | rebuild |
| apps/identity-api/migrations/0001_initial_schema.sql#users | `target-postgres:identity.users` | Identity | personal data | product-lifecycle-plus-privacy-retention | row_count, checksum, orphan_check | rebuild |
| apps/identity-api/migrations/0001_initial_schema.sql#workspace_invitations | `target-postgres:workspace-authz.workspace_invitations` | Workspace/Authz | personal data | product-lifecycle-plus-privacy-retention | row_count, checksum, orphan_check | rebuild |
| apps/identity-api/migrations/0001_initial_schema.sql#workspace_memberships | `target-postgres:workspace-authz.workspace_memberships` | Workspace/Authz | personal data | product-lifecycle-plus-privacy-retention | row_count, checksum, orphan_check | rebuild |
| apps/identity-api/migrations/0001_initial_schema.sql#workspace_policies | `target-postgres:workspace-authz.workspace_policies` | Workspace/Authz | personal data | product-lifecycle-plus-privacy-retention | row_count, checksum, orphan_check | rebuild |
| apps/identity-api/migrations/0001_initial_schema.sql#workspaces | `target-postgres:workspace-authz.workspaces` | Workspace/Authz | personal data | product-lifecycle-plus-privacy-retention | row_count, checksum, orphan_check | rebuild |
| apps/identity-api/migrations/0006_b2b_multi_tenant.sql#organization_invitations | `target-postgres:workspace-authz.organization_invitations` | Workspace/Authz | operational data | operational-retention | row_count, checksum | rebuild |
| apps/identity-api/migrations/0006_b2b_multi_tenant.sql#organization_policies | `target-postgres:workspace-authz.organization_policies` | Workspace/Authz | operational data | operational-retention | row_count, checksum | rebuild |
| apps/identity-api/migrations/0006_b2b_multi_tenant.sql#system_policies | `target-postgres:identity.system_policies` | Identity | operational data | operational-retention | row_count, checksum | rebuild |
| apps/identity-api/migrations/0006_b2b_multi_tenant.sql#tenant_invitations | `target-postgres:workspace-authz.tenant_invitations` | Workspace/Authz | personal data | product-lifecycle-plus-privacy-retention | row_count, checksum, orphan_check | rebuild |
| apps/identity-api/migrations/0006_b2b_multi_tenant.sql#tenant_policies | `target-postgres:workspace-authz.tenant_policies` | Workspace/Authz | personal data | product-lifecycle-plus-privacy-retention | row_count, checksum, orphan_check | rebuild |
| apps/identity-api/migrations/0008_developer_console.sql#developer_client_secret_versions | `target-postgres:developer-platform.developer_client_secret_versions` | Developer Platform | sensitive personal data | shortest-legal-retention | row_count | rebuild |
| apps/identity-api/migrations/0008_developer_console.sql#developer_consent_screens | `target-postgres:audit-privacy.developer_consent_screens` | Audit/Privacy | operational data | operational-retention | row_count, checksum | rebuild |
| apps/identity-api/migrations/0008_developer_console.sql#developer_health_checks | `target-postgres:developer-platform.developer_health_checks` | Developer Platform | operational data | operational-retention | row_count, checksum | rebuild |
| apps/identity-api/migrations/0008_developer_console.sql#developer_marketplace_apps | `target-postgres:developer-platform.developer_marketplace_apps` | Developer Platform | operational data | operational-retention | row_count, checksum | rebuild |
| apps/identity-api/migrations/0008_developer_console.sql#developer_role_assignments | `target-postgres:developer-platform.developer_role_assignments` | Developer Platform | operational data | operational-retention | row_count, checksum | rebuild |
| apps/identity-api/migrations/0008_developer_console.sql#developer_sandbox_tenants | `target-postgres:workspace-authz.developer_sandbox_tenants` | Workspace/Authz | personal data | product-lifecycle-plus-privacy-retention | row_count, checksum, orphan_check | rebuild |
| apps/identity-api/migrations/0008_developer_console.sql#developer_scope_registry | `target-postgres:developer-platform.developer_scope_registry` | Developer Platform | operational data | operational-retention | row_count, checksum | rebuild |
| apps/identity-api/migrations/0008_developer_console.sql#developer_secret_rotations | `target-postgres:developer-platform.developer_secret_rotations` | Developer Platform | sensitive personal data | shortest-legal-retention | row_count | rebuild |
| apps/identity-api/migrations/0008_developer_console.sql#developer_token_debug_sessions | `target-postgres:developer-platform.developer_token_debug_sessions` | Developer Platform | sensitive personal data | shortest-legal-retention | row_count | rebuild |
| apps/identity-api/migrations/0008_developer_console.sql#developer_webhook_deliveries | `target-postgres:developer-platform.developer_webhook_deliveries` | Developer Platform | operational data | operational-retention | row_count, checksum | rebuild |
| apps/identity-api/migrations/0008_developer_console.sql#developer_webhook_endpoints | `target-postgres:developer-platform.developer_webhook_endpoints` | Developer Platform | operational data | operational-retention | row_count, checksum | rebuild |
| apps/identity-api/migrations/0009_access_review_campaigns.sql#access_review_campaigns | `target-postgres:identity.access_review_campaigns` | Identity | operational data | operational-retention | row_count, checksum | rebuild |
| apps/identity-api/migrations/0009_access_review_campaigns.sql#access_review_items | `target-postgres:identity.access_review_items` | Identity | operational data | operational-retention | row_count, checksum | rebuild |
| apps/identity-api/migrations/0010_access_review_schedules.sql#access_review_schedules | `target-postgres:identity.access_review_schedules` | Identity | operational data | operational-retention | row_count, checksum | rebuild |
| apps/identity-api/migrations/0011_developer_portal.sql#developer_role_assignments | `target-postgres:developer-platform.developer_role_assignments` | Developer Platform | operational data | operational-retention | row_count, checksum | rebuild |
| apps/identity-api/migrations/0011_developer_portal.sql#developer_webhook_deliveries | `target-postgres:developer-platform.developer_webhook_deliveries` | Developer Platform | operational data | operational-retention | row_count, checksum | rebuild |
| apps/identity-api/migrations/0011_developer_portal.sql#developer_webhook_endpoints | `target-postgres:developer-platform.developer_webhook_endpoints` | Developer Platform | operational data | operational-retention | row_count, checksum | rebuild |
| apps/identity-api/migrations/0011_developer_portal.sql#developer_webhook_subscriptions | `target-postgres:billing-usage.developer_webhook_subscriptions` | Billing/Usage | financial data | contractual-financial-retention | row_count, checksum, ledger_balance | rebuild |
| apps/identity-api/migrations/0012_access_review_reminders.sql#access_review_reminders | `target-postgres:identity.access_review_reminders` | Identity | operational data | operational-retention | row_count, checksum | rebuild |
| apps/identity-api/migrations/0014_break_glass_accounts.sql#tenant_break_glass_accounts | `target-postgres:workspace-authz.tenant_break_glass_accounts` | Workspace/Authz | personal data | product-lifecycle-plus-privacy-retention | row_count, checksum, orphan_check | rebuild |

## Reject Classes

| Class | Description | Cutover impact |
|---|---|---|
| fixed | corrected before cutover | none |
| accepted | owner accepts migration gap | allowed only with evidence |
| rejected | excluded from migration | requires user/legal impact review |
| blocking | must stop cutover | no-go |

## Regeneration

```bash
pnpm check:migration-data-map
tools/migration/data-map.mjs --write
```
