---
title: Inventaire des APIs & Endpoints
description: Liste exhaustive des endpoints OpenAPI extraits des spécifications du projet.
---

> Ce catalogue est extrait directement des fichiers de spécification OpenAPI générés par les services Axum.

Total de services documentés : **9**

## nvbes Account Service (`0.1.0`)
- **Fichier source** : `apps/account-service-next/openapi.json`
- **Nombre d'endpoints** : **20**

| Méthode | Route | Description / OperationId | Tags |
| :--- | :--- | :--- | :--- |
| `GET` | `/api/v1/closure` | getAccountClosure | `closure` |
| `POST` | `/api/v1/closure` | requestAccountClosure | `closure` |
| `GET` | `/api/v1/consents` | listAccountConsents | `privacy` |
| `POST` | `/api/v1/consents` | grantAccountConsent | `privacy` |
| `DELETE` | `/api/v1/consents` | revokeAccountConsent | `privacy` |
| `GET` | `/api/v1/notifications` | getAccountNotifications | `settings` |
| `PUT` | `/api/v1/notifications` | updateAccountNotifications | `settings` |
| `GET` | `/api/v1/preferences` | getAccountPreferences | `settings` |
| `PUT` | `/api/v1/preferences` | updateAccountPreferences | `settings` |
| `POST` | `/api/v1/privacy/exports` | requestAccountPrivacyExport | `privacy` |
| `GET` | `/api/v1/privacy/exports/latest` | getLatestAccountPrivacyExport | `privacy` |
| `GET` | `/api/v1/privacy/exports/{exportId}/document` | downloadAccountPrivacyExport | `privacy` |
| `GET` | `/api/v1/privacy/gpc` | get_gpc_status | `privacy` |
| `GET` | `/api/v1/profile` | getAccountProfile | `profile` |
| `PUT` | `/api/v1/profile` | updateAccountProfile | `profile` |
| `GET` | `/api/v1/profile/avatar` | downloadAccountAvatar | `profile` |
| `POST` | `/api/v1/profile/avatar` | prepareAccountAvatarUpload | `profile` |
| `DELETE` | `/api/v1/profile/avatar` | deleteAccountAvatar | `profile` |
| `GET` | `/api/v1/security/sessions` | list_sessions | `security` |
| `DELETE` | `/api/v1/security/sessions/{sessionId}` | revoke_session | `security` |

## nvbes Backoffice API (`0.1.0`)
- **Fichier source** : `apps/backoffice-service/openapi.json`
- **Nombre d'endpoints** : **58**

| Méthode | Route | Description / OperationId | Tags |
| :--- | :--- | :--- | :--- |
| `POST` | `/admin/access-center/workspace-memberships/{workspaceId}/{principalId}/suspend` | Suspend workspace membership | `governance` |
| `GET` | `/admin/audit-evidence-center` | Load global audit evidence health | `audit` |
| `GET` | `/admin/command-center` | Load enterprise command center | `command` |
| `POST` | `/admin/identity-governance-center/break-glass/{tenantId}/{principalId}/revoke` | Revoke a break-glass account | `governance` |
| `POST` | `/admin/identity-governance-center/operator-grants/{principalId}/{role}/grant` | Grant a back-office operator role | `governance` |
| `POST` | `/admin/identity-governance-center/operator-grants/{principalId}/{role}/revoke` | Revoke a back-office operator role | `governance` |
| `POST` | `/admin/identity-governance-center/recovery-requests/{requestId}/cancel` | Cancel an enterprise recovery request | `governance` |
| `GET` | `/admin/pending-approvals` | List pending dual-control and review work | `command` |
| `POST` | `/admin/security-center/mfa-factors/{factorId}/revoke` | Revoke an MFA factor | `security` |
| `POST` | `/admin/security-center/oauth-consents/{consentId}/revoke` | Revoke an OAuth consent | `security` |
| `POST` | `/admin/tenants/{tenantId}/reactivate` | Reactivate a tenant | `governance` |
| `POST` | `/admin/tenants/{tenantId}/suspend` | Suspend a tenant | `governance` |
| `POST` | `/admin/users/{principalId}/reactivate` | Reactivate a user | `governance` |
| `POST` | `/admin/users/{principalId}/suspend` | Suspend a user | `governance` |
| `POST` | `/admin/workspaces/{workspaceId}/reactivate` | Reactivate a workspace | `governance` |
| `POST` | `/admin/workspaces/{workspaceId}/suspend` | Suspend a workspace | `governance` |
| `GET` | `/workspaces/{workspaceId}/admin/audit-events` | List enriched audit events for a workspace tenant | `audit` |
| `GET` | `/workspaces/{workspaceId}/admin/audit-evidence/export` | Export enriched audit evidence package for a workspace tenant | `audit` |
| `POST` | `/workspaces/{workspaceId}/admin/billing-platform/einvoicing-profiles/{profileId}/activate` | Activate an e-invoicing profile | `billing-platform` |
| `POST` | `/workspaces/{workspaceId}/admin/billing-platform/kyc-profiles/{profileId}/approve` | Approve a KYC profile | `billing-platform` |
| `POST` | `/workspaces/{workspaceId}/admin/billing-platform/kyc-profiles/{profileId}/reject` | Reject a KYC profile | `billing-platform` |
| `POST` | `/workspaces/{workspaceId}/admin/billing-platform/routing-rules` | Create a billing provider routing rule | `billing-platform` |
| `GET` | `/workspaces/{workspaceId}/admin/billing-platform/routing-rules/simulate` | Simulate billing provider routing rule matching | `billing-platform` |
| `POST` | `/workspaces/{workspaceId}/admin/billing-platform/routing-rules/{ruleId}/disable` | Disable a billing provider routing rule | `billing-platform` |
| `POST` | `/workspaces/{workspaceId}/admin/billing-platform/routing-rules/{ruleId}/enable` | Enable a billing provider routing rule | `billing-platform` |
| `POST` | `/workspaces/{workspaceId}/admin/communications/emails/{messageId}/replay` | Replay an email message | `communications` |
| `POST` | `/workspaces/{workspaceId}/admin/communications/suppressions` | Suppress an email address | `communications` |
| `POST` | `/workspaces/{workspaceId}/admin/communications/suppressions/remove` | Unsuppress an email address | `communications` |
| `POST` | `/workspaces/{workspaceId}/admin/compliance/consents/{consentId}/revoke` | Revoke a consent | `compliance` |
| `POST` | `/workspaces/{workspaceId}/admin/compliance/principals/{principalId}/erasure-request` | Request principal erasure | `compliance` |
| `POST` | `/workspaces/{workspaceId}/admin/compliance/suppressions/review` | Review email suppression | `compliance` |
| `POST` | `/workspaces/{workspaceId}/admin/developer/clients/{clientId}/revoke` | Revoke a developer client | `developer` |
| `POST` | `/workspaces/{workspaceId}/admin/developer/clients/{clientId}/rotate-secret` | Rotate a developer client secret | `developer` |
| `POST` | `/workspaces/{workspaceId}/admin/developer/marketplace-apps/{appId}/approve` | Approve a marketplace app | `developer` |
| `POST` | `/workspaces/{workspaceId}/admin/entitlements/grants` | Grant an entitlement feature | `entitlements` |
| `POST` | `/workspaces/{workspaceId}/admin/entitlements/publish` | Publish entitlement changes | `entitlements` |
| `POST` | `/workspaces/{workspaceId}/admin/entitlements/quota-overrides` | Override an entitlement quota | `entitlements` |
| `POST` | `/workspaces/{workspaceId}/admin/entitlements/revocations` | Revoke an entitlement feature | `entitlements` |
| `POST` | `/workspaces/{workspaceId}/admin/operations/export-runs/{exportRunId}/replay` | Replay an export run | `operations` |
| `POST` | `/workspaces/{workspaceId}/admin/operations/incidents/{incidentId}/state` | Update an operations incident state | `operations` |
| `POST` | `/workspaces/{workspaceId}/admin/operations/job-runs/{jobRunId}/replay` | Replay an operations job run | `operations` |
| `POST` | `/workspaces/{workspaceId}/admin/operations/maintenance-windows` | Schedule an operations maintenance window | `operations` |
| `POST` | `/workspaces/{workspaceId}/admin/operations/provider-events/{eventId}/replay` | Replay an operations provider event | `operations` |
| `POST` | `/workspaces/{workspaceId}/admin/operations/reconciliation-differences/{differenceId}/resolve` | Resolve a reconciliation difference | `operations` |
| `POST` | `/workspaces/{workspaceId}/admin/region/workspaces/{targetWorkspaceId}/exceptions` | Record a data residency exception | `region` |
| `POST` | `/workspaces/{workspaceId}/admin/region/workspaces/{targetWorkspaceId}/residency-flag` | Flag workspace data residency | `region` |
| `POST` | `/workspaces/{workspaceId}/admin/revenue/disputes/{disputeId}/resolve` | Resolve a revenue dispute | `revenue` |
| `POST` | `/workspaces/{workspaceId}/admin/revenue/disputes/{disputeId}/review` | Mark a revenue dispute as under review | `revenue` |
| `POST` | `/workspaces/{workspaceId}/admin/revenue/dunning-cases/{caseId}/close` | Close a dunning case | `revenue` |
| `POST` | `/workspaces/{workspaceId}/admin/revenue/dunning-cases/{caseId}/reopen` | Reopen a dunning case | `revenue` |
| `POST` | `/workspaces/{workspaceId}/admin/revenue/invoices/{invoiceId}/hold` | Hold an invoice | `revenue` |
| `POST` | `/workspaces/{workspaceId}/admin/revenue/invoices/{invoiceId}/release` | Release an invoice hold | `revenue` |
| `POST` | `/workspaces/{workspaceId}/admin/risk/policies/{policyId}/approve` | Approve a risk policy | `risk` |
| `POST` | `/workspaces/{workspaceId}/admin/risk/policies/{policyId}/block` | Block a risk policy | `risk` |
| `POST` | `/workspaces/{workspaceId}/admin/risk/signals/{signalId}/resolve` | Resolve a risk signal | `risk` |
| `POST` | `/workspaces/{workspaceId}/admin/usage/corrections` | Correct usage | `usage` |
| `POST` | `/workspaces/{workspaceId}/admin/usage/meters/freeze` | Freeze a usage meter | `usage` |
| `POST` | `/workspaces/{workspaceId}/admin/usage/rollups/{rollupId}/replay` | Replay a usage rollup | `usage` |

## nvbes Cloud Service (`0.1.0`)
- **Fichier source** : `apps/cloud-service/openapi.json`
- **Nombre d'endpoints** : **17**

| Méthode | Route | Description / OperationId | Tags |
| :--- | :--- | :--- | :--- |
| `GET` | `/v1/me` | me | `public-api` |
| `GET` | `/v1/workspaces` | list_workspaces | `public-api` |
| `GET` | `/v1/workspaces/{workspaceId}/audit-events` | list_audit_events | `public-api` |
| `POST` | `/v1/workspaces/{workspaceId}/folders` | create_folder | `public-api` |
| `GET` | `/v1/workspaces/{workspaceId}/objects` | list_objects | `public-api` |
| `PATCH` | `/v1/workspaces/{workspaceId}/objects/{objectId}` | rename_object | `public-api` |
| `POST` | `/v1/workspaces/{workspaceId}/objects/{objectId}/download-url` | create_download_url | `public-api` |
| `POST` | `/v1/workspaces/{workspaceId}/objects/{objectId}/move` | move_object | `public-api` |
| `POST` | `/v1/workspaces/{workspaceId}/objects/{objectId}/share-links` | create_share_link | `public-api` |
| `POST` | `/v1/workspaces/{workspaceId}/objects/{objectId}/trash` | trash_object | `public-api` |
| `GET` | `/v1/workspaces/{workspaceId}/quota` | get_quota | `public-api` |
| `GET` | `/v1/workspaces/{workspaceId}/share-links` | list_share_links | `public-api` |
| `DELETE` | `/v1/workspaces/{workspaceId}/share-links/{shareLinkId}` | revoke_share_link | `public-api` |
| `PATCH` | `/v1/workspaces/{workspaceId}/share-links/{shareLinkId}` | update_share_link | `public-api` |
| `POST` | `/v1/workspaces/{workspaceId}/uploads` | create_upload | `public-api` |
| `POST` | `/v1/workspaces/{workspaceId}/uploads/{uploadId}/cancel` | cancel_upload | `public-api` |
| `POST` | `/v1/workspaces/{workspaceId}/uploads/{uploadId}/complete` | complete_upload | `public-api` |

## nvbes Developer Service (`0.1.0`)
- **Fichier source** : `apps/developer-service/openapi.json`
- **Nombre d'endpoints** : **38**

| Méthode | Route | Description / OperationId | Tags |
| :--- | :--- | :--- | :--- |
| `GET` | `/developer/apps` | list_apps | `developer` |
| `POST` | `/developer/apps` | create_app | `developer` |
| `GET` | `/developer/apps/{clientId}` | get_app | `developer` |
| `DELETE` | `/developer/apps/{clientId}` | revoke_app | `developer` |
| `PATCH` | `/developer/apps/{clientId}/redirects` | update_redirects | `developer` |
| `GET` | `/developer/console/context` | context | `developer-console` |
| `GET` | `/developer/console/health-checks` | list_health_checks | `developer-console` |
| `POST` | `/developer/console/health-checks` | run_health_checks | `developer-console` |
| `GET` | `/developer/console/logs` | list_api_logs | `developer-console` |
| `GET` | `/developer/console/marketplace/apps` | list_marketplace_apps | `developer-console` |
| `POST` | `/developer/console/marketplace/apps/{clientId}/review` | review_marketplace_app | `developer-console` |
| `POST` | `/developer/console/marketplace/apps/{clientId}/submit` | submit_marketplace_app | `developer-console` |
| `GET` | `/developer/console/oauth-clients` | list_oauth_clients | `developer-console` |
| `GET` | `/developer/console/oauth-clients/{clientId}/consent-screen` | get_consent_screen | `developer-console` |
| `PUT` | `/developer/console/oauth-clients/{clientId}/consent-screen` | upsert_consent_screen | `developer-console` |
| `GET` | `/developer/console/oauth-clients/{clientId}/secrets` | list_secret_versions | `developer-console` |
| `POST` | `/developer/console/oauth-clients/{clientId}/secrets/rotation` | rotate_secret | `developer-console` |
| `POST` | `/developer/console/oauth-clients/{clientId}/secrets/{versionId}/revoke` | revoke_secret_version | `developer-console` |
| `GET` | `/developer/console/overview` | overview | `developer-console` |
| `GET` | `/developer/console/sandbox` | get_sandbox | `developer-console` |
| `PUT` | `/developer/console/sandbox` | upsert_sandbox | `developer-console` |
| `POST` | `/developer/console/sandbox/reset` | reset_sandbox | `developer-console` |
| `GET` | `/developer/console/scopes` | list_scopes | `developer-console` |
| `POST` | `/developer/console/scopes` | create_scope | `developer-console` |
| `PUT` | `/developer/console/scopes/{scopeKey}` | update_scope | `developer-console` |
| `DELETE` | `/developer/console/scopes/{scopeKey}` | delete_scope | `developer-console` |
| `GET` | `/developer/console/service-accounts` | list_service_accounts | `developer-console` |
| `POST` | `/developer/console/tokens/debug` | debug_token | `developer-console` |
| `GET` | `/developer/console/webhooks` | list_console_webhooks | `developer-console` |
| `POST` | `/developer/console/webhooks/deliveries/{deliveryId}/replay` | replay_delivery | `developer-console` |
| `GET` | `/developer/console/webhooks/{endpointId}/deliveries` | list_deliveries | `developer-console` |
| `GET` | `/developer/logs` | list_logs | `developer` |
| `GET` | `/developer/me` | me | `developer` |
| `POST` | `/developer/oauth/playground/exchange` | exchange_playground_code | `developer` |
| `POST` | `/developer/tokens/inspect` | inspect_token | `developer` |
| `GET` | `/developer/webhooks` | list_portal_webhooks | `developer` |
| `POST` | `/developer/webhooks` | create_portal_webhook | `developer` |
| `DELETE` | `/developer/webhooks/{endpointId}` | delete_portal_webhook | `developer` |

## nvbes Identity Service (`0.1.0`)
- **Fichier source** : `apps/identity-service/openapi.json`
- **Nombre d'endpoints** : **69**

| Méthode | Route | Description / OperationId | Tags |
| :--- | :--- | :--- | :--- |
| `GET` | `/.well-known/change-password` | change_password_well_known | `auth` |
| `GET` | `/.well-known/gpc.json` | gpc_well_known | `auth` |
| `GET` | `/.well-known/jwks.json` | get_jwks | `auth` |
| `GET` | `/.well-known/oauth-authorization-server` | oauth_authorization_server_metadata | `oauth` |
| `GET` | `/.well-known/openid-configuration` | openid_configuration | `oauth` |
| `GET` | `/.well-known/passkey-endpoints` | passkey_endpoints_well_known | `auth` |
| `GET` | `/.well-known/webauthn` | webauthn_well_known | `auth` |
| `GET` | `/auth/accounts` | get_accounts | `auth` |
| `POST` | `/auth/challenge/identifier` | challenge_identifier | `auth` |
| `POST` | `/auth/challenge/mfa` | challenge_mfa | `auth` |
| `GET` | `/auth/challenge/pow` | challenge_pow | `auth` |
| `POST` | `/auth/challenge/pwd` | challenge_pwd | `auth` |
| `POST` | `/auth/challenge/webauthn/discoverable/finish` | challenge_webauthn_discoverable_finish | `auth` |
| `POST` | `/auth/challenge/webauthn/discoverable/start` | challenge_webauthn_discoverable_start | `auth` |
| `POST` | `/auth/challenge/webauthn/start` | challenge_webauthn_start | `auth` |
| `DELETE` | `/auth/devices/{deviceId}` | revoke_device | `auth` |
| `POST` | `/auth/devices/{deviceId}/trust` | trust_device | `auth` |
| `POST` | `/auth/logout` | logout | `auth` |
| `GET` | `/auth/me/emails` | me_emails_get | `auth` |
| `POST` | `/auth/me/emails` | me_emails_post | `auth` |
| `DELETE` | `/auth/me/emails/{emailId}` | me_email_delete | `auth` |
| `POST` | `/auth/me/emails/{emailId}/promote` | me_email_promote | `auth` |
| `POST` | `/auth/me/emails/{emailId}/resend-verification` | me_email_resend_verification | `auth` |
| `GET` | `/auth/mfa/factors` | list_mfa_factors | `auth` |
| `DELETE` | `/auth/mfa/factors/{factorId}` | remove_mfa_factor | `auth` |
| `POST` | `/auth/mfa/recovery-codes` | generate_recovery_codes | `auth` |
| `POST` | `/auth/mfa/totp/confirm` | confirm_totp_enrollment | `auth` |
| `POST` | `/auth/mfa/totp/setup` | begin_totp_enrollment | `auth` |
| `POST` | `/auth/mfa/webauthn/register/finish` | confirm_webauthn_enrollment | `auth` |
| `POST` | `/auth/mfa/webauthn/register/start` | begin_webauthn_enrollment | `auth` |
| `POST` | `/auth/mfa/webauthn/start` | webauthn_auth_start | `auth` |
| `POST` | `/auth/password/change` | change_password | `auth` |
| `POST` | `/auth/password/forgot` | forgot_password | `auth` |
| `POST` | `/auth/password/reset` | reset_password | `auth` |
| `POST` | `/auth/register` | register | `auth` |
| `POST` | `/auth/registration/availability` | registration_availability | `auth` |
| `GET` | `/auth/sessions` | list_sessions | `auth` |
| `POST` | `/auth/sessions/revoke-all` | revoke_all_sessions | `auth` |
| `POST` | `/auth/sessions/revoke-others` | revoke_all_other_sessions | `auth` |
| `DELETE` | `/auth/sessions/{sessionId}` | revoke_session | `auth` |
| `POST` | `/auth/sessions/{sessionId}/confirm` | confirm_high_risk_session | `auth` |
| `POST` | `/auth/step-up` | step_up | `auth` |
| `POST` | `/auth/step-up/email/request` | request_email_step_up | `auth` |
| `POST` | `/auth/verify-email` | verify_email | `auth` |
| `POST` | `/auth/verify-email/change` | change_verify_email | `auth` |
| `POST` | `/auth/verify-email/resend` | resend_verify_email | `auth` |
| `POST` | `/auth/workspaces/{workspaceId}/switch` | switch_workspace | `auth` |
| `POST` | `/authz/decision` | decide | `authz` |
| `GET` | `/oauth/authorize` | authorize | `oauth` |
| `DELETE` | `/oauth/client-policies/{policyId}` | delete_client_policy | `oauth` |
| `PATCH` | `/oauth/client-policies/{policyId}` | update_client_policy | `oauth` |
| `GET` | `/oauth/clients` | list_clients | `oauth` |
| `POST` | `/oauth/clients` | create_client | `oauth` |
| `DELETE` | `/oauth/clients/{clientId}` | revoke_client | `oauth` |
| `GET` | `/oauth/clients/{clientId}/keys` | list_client_keys | `oauth` |
| `POST` | `/oauth/clients/{clientId}/keys` | rotate_client_key | `oauth` |
| `DELETE` | `/oauth/clients/{clientId}/keys/{keyId}` | revoke_client_key | `oauth` |
| `GET` | `/oauth/clients/{clientId}/policies` | list_client_policies | `oauth` |
| `POST` | `/oauth/clients/{clientId}/policies` | create_client_policy | `oauth` |
| `POST` | `/oauth/device/approve` | device_approve | `oauth` |
| `POST` | `/oauth/device/authorize` | device_authorize | `oauth` |
| `POST` | `/oauth/device/deny` | device_deny | `oauth` |
| `POST` | `/oauth/device/verify` | device_verify | `oauth` |
| `POST` | `/oauth/introspect` | introspect | `oauth` |
| `POST` | `/oauth/revoke` | revoke | `oauth` |
| `POST` | `/oauth/token` | token | `oauth` |
| `GET` | `/oauth/userinfo` | userinfo | `oauth` |
| `GET` | `/workspaces/{workspaceId}/security-events` | list_security_events | `security` |
| `GET` | `/workspaces/{workspaceId}/security-events/export` | export_security_events | `security` |

## nvbes Cloud API (`0.1.0`)
- **Fichier source** : `docs/api/openapi/cloud-public-v1.openapi.json`
- **Nombre d'endpoints** : **17**

| Méthode | Route | Description / OperationId | Tags |
| :--- | :--- | :--- | :--- |
| `GET` | `/v1/me` | me | `public-api` |
| `GET` | `/v1/workspaces` | list_workspaces | `public-api` |
| `GET` | `/v1/workspaces/{workspaceId}/audit-events` | list_audit_events | `public-api` |
| `POST` | `/v1/workspaces/{workspaceId}/folders` | create_folder | `public-api` |
| `GET` | `/v1/workspaces/{workspaceId}/objects` | list_objects | `public-api` |
| `PATCH` | `/v1/workspaces/{workspaceId}/objects/{objectId}` | rename_object | `public-api` |
| `POST` | `/v1/workspaces/{workspaceId}/objects/{objectId}/download-url` | create_download_url | `public-api` |
| `POST` | `/v1/workspaces/{workspaceId}/objects/{objectId}/move` | move_object | `public-api` |
| `POST` | `/v1/workspaces/{workspaceId}/objects/{objectId}/share-links` | create_share_link | `public-api` |
| `POST` | `/v1/workspaces/{workspaceId}/objects/{objectId}/trash` | trash_object | `public-api` |
| `GET` | `/v1/workspaces/{workspaceId}/quota` | get_quota | `public-api` |
| `GET` | `/v1/workspaces/{workspaceId}/share-links` | list_share_links | `public-api` |
| `DELETE` | `/v1/workspaces/{workspaceId}/share-links/{shareLinkId}` | revoke_share_link | `public-api` |
| `PATCH` | `/v1/workspaces/{workspaceId}/share-links/{shareLinkId}` | update_share_link | `public-api` |
| `POST` | `/v1/workspaces/{workspaceId}/uploads` | create_upload | `public-api` |
| `POST` | `/v1/workspaces/{workspaceId}/uploads/{uploadId}/cancel` | cancel_upload | `public-api` |
| `POST` | `/v1/workspaces/{workspaceId}/uploads/{uploadId}/complete` | complete_upload | `public-api` |

## nvbes Cloud API (`0.1.0`)
- **Fichier source** : `docs/api/openapi/drive-public-v1.openapi.json`
- **Nombre d'endpoints** : **17**

| Méthode | Route | Description / OperationId | Tags |
| :--- | :--- | :--- | :--- |
| `GET` | `/v1/me` | me | `public-api` |
| `GET` | `/v1/workspaces` | list_workspaces | `public-api` |
| `GET` | `/v1/workspaces/{workspaceId}/audit-events` | list_audit_events | `public-api` |
| `POST` | `/v1/workspaces/{workspaceId}/folders` | create_folder | `public-api` |
| `GET` | `/v1/workspaces/{workspaceId}/objects` | list_objects | `public-api` |
| `PATCH` | `/v1/workspaces/{workspaceId}/objects/{objectId}` | rename_object | `public-api` |
| `POST` | `/v1/workspaces/{workspaceId}/objects/{objectId}/download-url` | create_download_url | `public-api` |
| `POST` | `/v1/workspaces/{workspaceId}/objects/{objectId}/move` | move_object | `public-api` |
| `POST` | `/v1/workspaces/{workspaceId}/objects/{objectId}/share-links` | create_share_link | `public-api` |
| `POST` | `/v1/workspaces/{workspaceId}/objects/{objectId}/trash` | trash_object | `public-api` |
| `GET` | `/v1/workspaces/{workspaceId}/quota` | get_quota | `public-api` |
| `GET` | `/v1/workspaces/{workspaceId}/share-links` | list_share_links | `public-api` |
| `DELETE` | `/v1/workspaces/{workspaceId}/share-links/{shareLinkId}` | revoke_share_link | `public-api` |
| `PATCH` | `/v1/workspaces/{workspaceId}/share-links/{shareLinkId}` | update_share_link | `public-api` |
| `POST` | `/v1/workspaces/{workspaceId}/uploads` | create_upload | `public-api` |
| `POST` | `/v1/workspaces/{workspaceId}/uploads/{uploadId}/cancel` | cancel_upload | `public-api` |
| `POST` | `/v1/workspaces/{workspaceId}/uploads/{uploadId}/complete` | complete_upload | `public-api` |

## nvbes Backoffice API (`0.1.0`)
- **Fichier source** : `libs/ts/backoffice-service-sdk-core/openapi.json`
- **Nombre d'endpoints** : **58**

| Méthode | Route | Description / OperationId | Tags |
| :--- | :--- | :--- | :--- |
| `POST` | `/admin/access-center/workspace-memberships/{workspaceId}/{principalId}/suspend` | Suspend workspace membership | `governance` |
| `GET` | `/admin/audit-evidence-center` | Load global audit evidence health | `audit` |
| `GET` | `/admin/command-center` | Load enterprise command center | `command` |
| `POST` | `/admin/identity-governance-center/break-glass/{tenantId}/{principalId}/revoke` | Revoke a break-glass account | `governance` |
| `POST` | `/admin/identity-governance-center/operator-grants/{principalId}/{role}/grant` | Grant a back-office operator role | `governance` |
| `POST` | `/admin/identity-governance-center/operator-grants/{principalId}/{role}/revoke` | Revoke a back-office operator role | `governance` |
| `POST` | `/admin/identity-governance-center/recovery-requests/{requestId}/cancel` | Cancel an enterprise recovery request | `governance` |
| `GET` | `/admin/pending-approvals` | List pending dual-control and review work | `command` |
| `POST` | `/admin/security-center/mfa-factors/{factorId}/revoke` | Revoke an MFA factor | `security` |
| `POST` | `/admin/security-center/oauth-consents/{consentId}/revoke` | Revoke an OAuth consent | `security` |
| `POST` | `/admin/tenants/{tenantId}/reactivate` | Reactivate a tenant | `governance` |
| `POST` | `/admin/tenants/{tenantId}/suspend` | Suspend a tenant | `governance` |
| `POST` | `/admin/users/{principalId}/reactivate` | Reactivate a user | `governance` |
| `POST` | `/admin/users/{principalId}/suspend` | Suspend a user | `governance` |
| `POST` | `/admin/workspaces/{workspaceId}/reactivate` | Reactivate a workspace | `governance` |
| `POST` | `/admin/workspaces/{workspaceId}/suspend` | Suspend a workspace | `governance` |
| `GET` | `/workspaces/{workspaceId}/admin/audit-events` | List enriched audit events for a workspace tenant | `audit` |
| `GET` | `/workspaces/{workspaceId}/admin/audit-evidence/export` | Export enriched audit evidence package for a workspace tenant | `audit` |
| `POST` | `/workspaces/{workspaceId}/admin/billing-platform/einvoicing-profiles/{profileId}/activate` | Activate an e-invoicing profile | `billing-platform` |
| `POST` | `/workspaces/{workspaceId}/admin/billing-platform/kyc-profiles/{profileId}/approve` | Approve a KYC profile | `billing-platform` |
| `POST` | `/workspaces/{workspaceId}/admin/billing-platform/kyc-profiles/{profileId}/reject` | Reject a KYC profile | `billing-platform` |
| `POST` | `/workspaces/{workspaceId}/admin/billing-platform/routing-rules` | Create a billing provider routing rule | `billing-platform` |
| `GET` | `/workspaces/{workspaceId}/admin/billing-platform/routing-rules/simulate` | Simulate billing provider routing rule matching | `billing-platform` |
| `POST` | `/workspaces/{workspaceId}/admin/billing-platform/routing-rules/{ruleId}/disable` | Disable a billing provider routing rule | `billing-platform` |
| `POST` | `/workspaces/{workspaceId}/admin/billing-platform/routing-rules/{ruleId}/enable` | Enable a billing provider routing rule | `billing-platform` |
| `POST` | `/workspaces/{workspaceId}/admin/communications/emails/{messageId}/replay` | Replay an email message | `communications` |
| `POST` | `/workspaces/{workspaceId}/admin/communications/suppressions` | Suppress an email address | `communications` |
| `POST` | `/workspaces/{workspaceId}/admin/communications/suppressions/remove` | Unsuppress an email address | `communications` |
| `POST` | `/workspaces/{workspaceId}/admin/compliance/consents/{consentId}/revoke` | Revoke a consent | `compliance` |
| `POST` | `/workspaces/{workspaceId}/admin/compliance/principals/{principalId}/erasure-request` | Request principal erasure | `compliance` |
| `POST` | `/workspaces/{workspaceId}/admin/compliance/suppressions/review` | Review email suppression | `compliance` |
| `POST` | `/workspaces/{workspaceId}/admin/developer/clients/{clientId}/revoke` | Revoke a developer client | `developer` |
| `POST` | `/workspaces/{workspaceId}/admin/developer/clients/{clientId}/rotate-secret` | Rotate a developer client secret | `developer` |
| `POST` | `/workspaces/{workspaceId}/admin/developer/marketplace-apps/{appId}/approve` | Approve a marketplace app | `developer` |
| `POST` | `/workspaces/{workspaceId}/admin/entitlements/grants` | Grant an entitlement feature | `entitlements` |
| `POST` | `/workspaces/{workspaceId}/admin/entitlements/publish` | Publish entitlement changes | `entitlements` |
| `POST` | `/workspaces/{workspaceId}/admin/entitlements/quota-overrides` | Override an entitlement quota | `entitlements` |
| `POST` | `/workspaces/{workspaceId}/admin/entitlements/revocations` | Revoke an entitlement feature | `entitlements` |
| `POST` | `/workspaces/{workspaceId}/admin/operations/export-runs/{exportRunId}/replay` | Replay an export run | `operations` |
| `POST` | `/workspaces/{workspaceId}/admin/operations/incidents/{incidentId}/state` | Update an operations incident state | `operations` |
| `POST` | `/workspaces/{workspaceId}/admin/operations/job-runs/{jobRunId}/replay` | Replay an operations job run | `operations` |
| `POST` | `/workspaces/{workspaceId}/admin/operations/maintenance-windows` | Schedule an operations maintenance window | `operations` |
| `POST` | `/workspaces/{workspaceId}/admin/operations/provider-events/{eventId}/replay` | Replay an operations provider event | `operations` |
| `POST` | `/workspaces/{workspaceId}/admin/operations/reconciliation-differences/{differenceId}/resolve` | Resolve a reconciliation difference | `operations` |
| `POST` | `/workspaces/{workspaceId}/admin/region/workspaces/{targetWorkspaceId}/exceptions` | Record a data residency exception | `region` |
| `POST` | `/workspaces/{workspaceId}/admin/region/workspaces/{targetWorkspaceId}/residency-flag` | Flag workspace data residency | `region` |
| `POST` | `/workspaces/{workspaceId}/admin/revenue/disputes/{disputeId}/resolve` | Resolve a revenue dispute | `revenue` |
| `POST` | `/workspaces/{workspaceId}/admin/revenue/disputes/{disputeId}/review` | Mark a revenue dispute as under review | `revenue` |
| `POST` | `/workspaces/{workspaceId}/admin/revenue/dunning-cases/{caseId}/close` | Close a dunning case | `revenue` |
| `POST` | `/workspaces/{workspaceId}/admin/revenue/dunning-cases/{caseId}/reopen` | Reopen a dunning case | `revenue` |
| `POST` | `/workspaces/{workspaceId}/admin/revenue/invoices/{invoiceId}/hold` | Hold an invoice | `revenue` |
| `POST` | `/workspaces/{workspaceId}/admin/revenue/invoices/{invoiceId}/release` | Release an invoice hold | `revenue` |
| `POST` | `/workspaces/{workspaceId}/admin/risk/policies/{policyId}/approve` | Approve a risk policy | `risk` |
| `POST` | `/workspaces/{workspaceId}/admin/risk/policies/{policyId}/block` | Block a risk policy | `risk` |
| `POST` | `/workspaces/{workspaceId}/admin/risk/signals/{signalId}/resolve` | Resolve a risk signal | `risk` |
| `POST` | `/workspaces/{workspaceId}/admin/usage/corrections` | Correct usage | `usage` |
| `POST` | `/workspaces/{workspaceId}/admin/usage/meters/freeze` | Freeze a usage meter | `usage` |
| `POST` | `/workspaces/{workspaceId}/admin/usage/rollups/{rollupId}/replay` | Replay a usage rollup | `usage` |

## nvbes Identity Service (`0.1.0`)
- **Fichier source** : `libs/ts/identity-sdk-core/openapi.json`
- **Nombre d'endpoints** : **69**

| Méthode | Route | Description / OperationId | Tags |
| :--- | :--- | :--- | :--- |
| `GET` | `/.well-known/change-password` | change_password_well_known | `auth` |
| `GET` | `/.well-known/gpc.json` | gpc_well_known | `auth` |
| `GET` | `/.well-known/jwks.json` | get_jwks | `auth` |
| `GET` | `/.well-known/oauth-authorization-server` | oauth_authorization_server_metadata | `oauth` |
| `GET` | `/.well-known/openid-configuration` | openid_configuration | `oauth` |
| `GET` | `/.well-known/passkey-endpoints` | passkey_endpoints_well_known | `auth` |
| `GET` | `/.well-known/webauthn` | webauthn_well_known | `auth` |
| `GET` | `/auth/accounts` | get_accounts | `auth` |
| `POST` | `/auth/challenge/identifier` | challenge_identifier | `auth` |
| `POST` | `/auth/challenge/mfa` | challenge_mfa | `auth` |
| `GET` | `/auth/challenge/pow` | challenge_pow | `auth` |
| `POST` | `/auth/challenge/pwd` | challenge_pwd | `auth` |
| `POST` | `/auth/challenge/webauthn/discoverable/finish` | challenge_webauthn_discoverable_finish | `auth` |
| `POST` | `/auth/challenge/webauthn/discoverable/start` | challenge_webauthn_discoverable_start | `auth` |
| `POST` | `/auth/challenge/webauthn/start` | challenge_webauthn_start | `auth` |
| `DELETE` | `/auth/devices/{deviceId}` | revoke_device | `auth` |
| `POST` | `/auth/devices/{deviceId}/trust` | trust_device | `auth` |
| `POST` | `/auth/logout` | logout | `auth` |
| `GET` | `/auth/me/emails` | me_emails_get | `auth` |
| `POST` | `/auth/me/emails` | me_emails_post | `auth` |
| `DELETE` | `/auth/me/emails/{emailId}` | me_email_delete | `auth` |
| `POST` | `/auth/me/emails/{emailId}/promote` | me_email_promote | `auth` |
| `POST` | `/auth/me/emails/{emailId}/resend-verification` | me_email_resend_verification | `auth` |
| `GET` | `/auth/mfa/factors` | list_mfa_factors | `auth` |
| `DELETE` | `/auth/mfa/factors/{factorId}` | remove_mfa_factor | `auth` |
| `POST` | `/auth/mfa/recovery-codes` | generate_recovery_codes | `auth` |
| `POST` | `/auth/mfa/totp/confirm` | confirm_totp_enrollment | `auth` |
| `POST` | `/auth/mfa/totp/setup` | begin_totp_enrollment | `auth` |
| `POST` | `/auth/mfa/webauthn/register/finish` | confirm_webauthn_enrollment | `auth` |
| `POST` | `/auth/mfa/webauthn/register/start` | begin_webauthn_enrollment | `auth` |
| `POST` | `/auth/mfa/webauthn/start` | webauthn_auth_start | `auth` |
| `POST` | `/auth/password/change` | change_password | `auth` |
| `POST` | `/auth/password/forgot` | forgot_password | `auth` |
| `POST` | `/auth/password/reset` | reset_password | `auth` |
| `POST` | `/auth/register` | register | `auth` |
| `POST` | `/auth/registration/availability` | registration_availability | `auth` |
| `GET` | `/auth/sessions` | list_sessions | `auth` |
| `POST` | `/auth/sessions/revoke-all` | revoke_all_sessions | `auth` |
| `POST` | `/auth/sessions/revoke-others` | revoke_all_other_sessions | `auth` |
| `DELETE` | `/auth/sessions/{sessionId}` | revoke_session | `auth` |
| `POST` | `/auth/sessions/{sessionId}/confirm` | confirm_high_risk_session | `auth` |
| `POST` | `/auth/step-up` | step_up | `auth` |
| `POST` | `/auth/step-up/email/request` | request_email_step_up | `auth` |
| `POST` | `/auth/verify-email` | verify_email | `auth` |
| `POST` | `/auth/verify-email/change` | change_verify_email | `auth` |
| `POST` | `/auth/verify-email/resend` | resend_verify_email | `auth` |
| `POST` | `/auth/workspaces/{workspaceId}/switch` | switch_workspace | `auth` |
| `POST` | `/authz/decision` | decide | `authz` |
| `GET` | `/oauth/authorize` | authorize | `oauth` |
| `DELETE` | `/oauth/client-policies/{policyId}` | delete_client_policy | `oauth` |
| `PATCH` | `/oauth/client-policies/{policyId}` | update_client_policy | `oauth` |
| `GET` | `/oauth/clients` | list_clients | `oauth` |
| `POST` | `/oauth/clients` | create_client | `oauth` |
| `DELETE` | `/oauth/clients/{clientId}` | revoke_client | `oauth` |
| `GET` | `/oauth/clients/{clientId}/keys` | list_client_keys | `oauth` |
| `POST` | `/oauth/clients/{clientId}/keys` | rotate_client_key | `oauth` |
| `DELETE` | `/oauth/clients/{clientId}/keys/{keyId}` | revoke_client_key | `oauth` |
| `GET` | `/oauth/clients/{clientId}/policies` | list_client_policies | `oauth` |
| `POST` | `/oauth/clients/{clientId}/policies` | create_client_policy | `oauth` |
| `POST` | `/oauth/device/approve` | device_approve | `oauth` |
| `POST` | `/oauth/device/authorize` | device_authorize | `oauth` |
| `POST` | `/oauth/device/deny` | device_deny | `oauth` |
| `POST` | `/oauth/device/verify` | device_verify | `oauth` |
| `POST` | `/oauth/introspect` | introspect | `oauth` |
| `POST` | `/oauth/revoke` | revoke | `oauth` |
| `POST` | `/oauth/token` | token | `oauth` |
| `GET` | `/oauth/userinfo` | userinfo | `oauth` |
| `GET` | `/workspaces/{workspaceId}/security-events` | list_security_events | `security` |
| `GET` | `/workspaces/{workspaceId}/security-events/export` | export_security_events | `security` |

