---
title: API nvbes Backoffice API
description: Contrats OpenAPI, routes, paramètres et schémas pour nvbes Backoffice API (0.1.0).
---

> Source : `apps/backoffice-service/openapi.json` • Version : `0.1.0` • Endpoints : **58**

## Endpoints Disponibles

### `POST` `/admin/access-center/workspace-memberships/{workspaceId}/{principalId}/suspend`
- **Description** : Suspend workspace membership
- **Sécurité** : 🌐 Public
- **Tags** : `governance`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `GET` `/admin/audit-evidence-center`
- **Description** : Load global audit evidence health
- **Sécurité** : 🌐 Public
- **Tags** : `audit`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `GET` `/admin/command-center`
- **Description** : Load enterprise command center
- **Sécurité** : 🌐 Public
- **Tags** : `command`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `POST` `/admin/identity-governance-center/break-glass/{tenantId}/{principalId}/revoke`
- **Description** : Revoke a break-glass account
- **Sécurité** : 🌐 Public
- **Tags** : `governance`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `POST` `/admin/identity-governance-center/operator-grants/{principalId}/{role}/grant`
- **Description** : Grant a back-office operator role
- **Sécurité** : 🌐 Public
- **Tags** : `governance`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `POST` `/admin/identity-governance-center/operator-grants/{principalId}/{role}/revoke`
- **Description** : Revoke a back-office operator role
- **Sécurité** : 🌐 Public
- **Tags** : `governance`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `POST` `/admin/identity-governance-center/recovery-requests/{requestId}/cancel`
- **Description** : Cancel an enterprise recovery request
- **Sécurité** : 🌐 Public
- **Tags** : `governance`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `GET` `/admin/pending-approvals`
- **Description** : List pending dual-control and review work
- **Sécurité** : 🌐 Public
- **Tags** : `command`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `POST` `/admin/security-center/mfa-factors/{factorId}/revoke`
- **Description** : Revoke an MFA factor
- **Sécurité** : 🌐 Public
- **Tags** : `security`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `POST` `/admin/security-center/oauth-consents/{consentId}/revoke`
- **Description** : Revoke an OAuth consent
- **Sécurité** : 🌐 Public
- **Tags** : `security`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `POST` `/admin/tenants/{tenantId}/reactivate`
- **Description** : Reactivate a tenant
- **Sécurité** : 🌐 Public
- **Tags** : `governance`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `POST` `/admin/tenants/{tenantId}/suspend`
- **Description** : Suspend a tenant
- **Sécurité** : 🌐 Public
- **Tags** : `governance`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `POST` `/admin/users/{principalId}/reactivate`
- **Description** : Reactivate a user
- **Sécurité** : 🌐 Public
- **Tags** : `governance`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `POST` `/admin/users/{principalId}/suspend`
- **Description** : Suspend a user
- **Sécurité** : 🌐 Public
- **Tags** : `governance`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `POST` `/admin/workspaces/{workspaceId}/reactivate`
- **Description** : Reactivate a workspace
- **Sécurité** : 🌐 Public
- **Tags** : `governance`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `POST` `/admin/workspaces/{workspaceId}/suspend`
- **Description** : Suspend a workspace
- **Sécurité** : 🌐 Public
- **Tags** : `governance`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `GET` `/workspaces/{workspaceId}/admin/audit-events`
- **Description** : List enriched audit events for a workspace tenant
- **Sécurité** : 🌐 Public
- **Tags** : `audit`
- **Codes Retour** : `200`, `401`, `403`, `429`

---
### `GET` `/workspaces/{workspaceId}/admin/audit-evidence/export`
- **Description** : Export enriched audit evidence package for a workspace tenant
- **Sécurité** : 🌐 Public
- **Tags** : `audit`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `POST` `/workspaces/{workspaceId}/admin/billing-platform/einvoicing-profiles/{profileId}/activate`
- **Description** : Activate an e-invoicing profile
- **Sécurité** : 🌐 Public
- **Tags** : `billing-platform`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `POST` `/workspaces/{workspaceId}/admin/billing-platform/kyc-profiles/{profileId}/approve`
- **Description** : Approve a KYC profile
- **Sécurité** : 🌐 Public
- **Tags** : `billing-platform`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `POST` `/workspaces/{workspaceId}/admin/billing-platform/kyc-profiles/{profileId}/reject`
- **Description** : Reject a KYC profile
- **Sécurité** : 🌐 Public
- **Tags** : `billing-platform`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `POST` `/workspaces/{workspaceId}/admin/billing-platform/routing-rules`
- **Description** : Create a billing provider routing rule
- **Sécurité** : 🌐 Public
- **Tags** : `billing-platform`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `GET` `/workspaces/{workspaceId}/admin/billing-platform/routing-rules/simulate`
- **Description** : Simulate billing provider routing rule matching
- **Sécurité** : 🌐 Public
- **Tags** : `billing-platform`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `POST` `/workspaces/{workspaceId}/admin/billing-platform/routing-rules/{ruleId}/disable`
- **Description** : Disable a billing provider routing rule
- **Sécurité** : 🌐 Public
- **Tags** : `billing-platform`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `POST` `/workspaces/{workspaceId}/admin/billing-platform/routing-rules/{ruleId}/enable`
- **Description** : Enable a billing provider routing rule
- **Sécurité** : 🌐 Public
- **Tags** : `billing-platform`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `POST` `/workspaces/{workspaceId}/admin/communications/emails/{messageId}/replay`
- **Description** : Replay an email message
- **Sécurité** : 🌐 Public
- **Tags** : `communications`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `POST` `/workspaces/{workspaceId}/admin/communications/suppressions`
- **Description** : Suppress an email address
- **Sécurité** : 🌐 Public
- **Tags** : `communications`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `POST` `/workspaces/{workspaceId}/admin/communications/suppressions/remove`
- **Description** : Unsuppress an email address
- **Sécurité** : 🌐 Public
- **Tags** : `communications`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `POST` `/workspaces/{workspaceId}/admin/compliance/consents/{consentId}/revoke`
- **Description** : Revoke a consent
- **Sécurité** : 🌐 Public
- **Tags** : `compliance`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `POST` `/workspaces/{workspaceId}/admin/compliance/principals/{principalId}/erasure-request`
- **Description** : Request principal erasure
- **Sécurité** : 🌐 Public
- **Tags** : `compliance`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `POST` `/workspaces/{workspaceId}/admin/compliance/suppressions/review`
- **Description** : Review email suppression
- **Sécurité** : 🌐 Public
- **Tags** : `compliance`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `POST` `/workspaces/{workspaceId}/admin/developer/clients/{clientId}/revoke`
- **Description** : Revoke a developer client
- **Sécurité** : 🌐 Public
- **Tags** : `developer`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `POST` `/workspaces/{workspaceId}/admin/developer/clients/{clientId}/rotate-secret`
- **Description** : Rotate a developer client secret
- **Sécurité** : 🌐 Public
- **Tags** : `developer`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `POST` `/workspaces/{workspaceId}/admin/developer/marketplace-apps/{appId}/approve`
- **Description** : Approve a marketplace app
- **Sécurité** : 🌐 Public
- **Tags** : `developer`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `POST` `/workspaces/{workspaceId}/admin/entitlements/grants`
- **Description** : Grant an entitlement feature
- **Sécurité** : 🌐 Public
- **Tags** : `entitlements`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `POST` `/workspaces/{workspaceId}/admin/entitlements/publish`
- **Description** : Publish entitlement changes
- **Sécurité** : 🌐 Public
- **Tags** : `entitlements`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `POST` `/workspaces/{workspaceId}/admin/entitlements/quota-overrides`
- **Description** : Override an entitlement quota
- **Sécurité** : 🌐 Public
- **Tags** : `entitlements`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `POST` `/workspaces/{workspaceId}/admin/entitlements/revocations`
- **Description** : Revoke an entitlement feature
- **Sécurité** : 🌐 Public
- **Tags** : `entitlements`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `POST` `/workspaces/{workspaceId}/admin/operations/export-runs/{exportRunId}/replay`
- **Description** : Replay an export run
- **Sécurité** : 🌐 Public
- **Tags** : `operations`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `POST` `/workspaces/{workspaceId}/admin/operations/incidents/{incidentId}/state`
- **Description** : Update an operations incident state
- **Sécurité** : 🌐 Public
- **Tags** : `operations`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `POST` `/workspaces/{workspaceId}/admin/operations/job-runs/{jobRunId}/replay`
- **Description** : Replay an operations job run
- **Sécurité** : 🌐 Public
- **Tags** : `operations`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `POST` `/workspaces/{workspaceId}/admin/operations/maintenance-windows`
- **Description** : Schedule an operations maintenance window
- **Sécurité** : 🌐 Public
- **Tags** : `operations`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `POST` `/workspaces/{workspaceId}/admin/operations/provider-events/{eventId}/replay`
- **Description** : Replay an operations provider event
- **Sécurité** : 🌐 Public
- **Tags** : `operations`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `POST` `/workspaces/{workspaceId}/admin/operations/reconciliation-differences/{differenceId}/resolve`
- **Description** : Resolve a reconciliation difference
- **Sécurité** : 🌐 Public
- **Tags** : `operations`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `POST` `/workspaces/{workspaceId}/admin/region/workspaces/{targetWorkspaceId}/exceptions`
- **Description** : Record a data residency exception
- **Sécurité** : 🌐 Public
- **Tags** : `region`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `POST` `/workspaces/{workspaceId}/admin/region/workspaces/{targetWorkspaceId}/residency-flag`
- **Description** : Flag workspace data residency
- **Sécurité** : 🌐 Public
- **Tags** : `region`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `POST` `/workspaces/{workspaceId}/admin/revenue/disputes/{disputeId}/resolve`
- **Description** : Resolve a revenue dispute
- **Sécurité** : 🌐 Public
- **Tags** : `revenue`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `POST` `/workspaces/{workspaceId}/admin/revenue/disputes/{disputeId}/review`
- **Description** : Mark a revenue dispute as under review
- **Sécurité** : 🌐 Public
- **Tags** : `revenue`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `POST` `/workspaces/{workspaceId}/admin/revenue/dunning-cases/{caseId}/close`
- **Description** : Close a dunning case
- **Sécurité** : 🌐 Public
- **Tags** : `revenue`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `POST` `/workspaces/{workspaceId}/admin/revenue/dunning-cases/{caseId}/reopen`
- **Description** : Reopen a dunning case
- **Sécurité** : 🌐 Public
- **Tags** : `revenue`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `POST` `/workspaces/{workspaceId}/admin/revenue/invoices/{invoiceId}/hold`
- **Description** : Hold an invoice
- **Sécurité** : 🌐 Public
- **Tags** : `revenue`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `POST` `/workspaces/{workspaceId}/admin/revenue/invoices/{invoiceId}/release`
- **Description** : Release an invoice hold
- **Sécurité** : 🌐 Public
- **Tags** : `revenue`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `POST` `/workspaces/{workspaceId}/admin/risk/policies/{policyId}/approve`
- **Description** : Approve a risk policy
- **Sécurité** : 🌐 Public
- **Tags** : `risk`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `POST` `/workspaces/{workspaceId}/admin/risk/policies/{policyId}/block`
- **Description** : Block a risk policy
- **Sécurité** : 🌐 Public
- **Tags** : `risk`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `POST` `/workspaces/{workspaceId}/admin/risk/signals/{signalId}/resolve`
- **Description** : Resolve a risk signal
- **Sécurité** : 🌐 Public
- **Tags** : `risk`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `POST` `/workspaces/{workspaceId}/admin/usage/corrections`
- **Description** : Correct usage
- **Sécurité** : 🌐 Public
- **Tags** : `usage`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `POST` `/workspaces/{workspaceId}/admin/usage/meters/freeze`
- **Description** : Freeze a usage meter
- **Sécurité** : 🌐 Public
- **Tags** : `usage`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
### `POST` `/workspaces/{workspaceId}/admin/usage/rollups/{rollupId}/replay`
- **Description** : Replay a usage rollup
- **Sécurité** : 🌐 Public
- **Tags** : `usage`
- **Codes Retour** : `200`, `400`, `401`, `403`, `429`

---
