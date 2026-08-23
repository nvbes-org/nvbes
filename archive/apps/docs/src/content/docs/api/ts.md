---
title: API nvbes Identity Service
description: Contrats OpenAPI, routes, paramètres et schémas pour nvbes Identity Service (0.1.0).
---

> Source : `libs/ts/identity-sdk-core/openapi.json` • Version : `0.1.0` • Endpoints : **69**

## Endpoints Disponibles

### `GET` `/.well-known/change-password`
- **Description** : change_password_well_known
- **Sécurité** : 🌐 Public
- **Tags** : `auth`
- **Codes Retour** : `200`, `500`

---
### `GET` `/.well-known/gpc.json`
- **Description** : gpc_well_known
- **Sécurité** : 🌐 Public
- **Tags** : `auth`
- **Codes Retour** : `200`

---
### `GET` `/.well-known/jwks.json`
- **Description** : get_jwks
- **Sécurité** : 🌐 Public
- **Tags** : `auth`
- **Codes Retour** : `200`, `500`

---
### `GET` `/.well-known/oauth-authorization-server`
- **Description** : oauth_authorization_server_metadata
- **Sécurité** : 🌐 Public
- **Tags** : `oauth`
- **Codes Retour** : `200`, `500`

---
### `GET` `/.well-known/openid-configuration`
- **Description** : openid_configuration
- **Sécurité** : 🌐 Public
- **Tags** : `oauth`
- **Codes Retour** : `200`, `500`

---
### `GET` `/.well-known/passkey-endpoints`
- **Description** : passkey_endpoints_well_known
- **Sécurité** : 🌐 Public
- **Tags** : `auth`
- **Codes Retour** : `200`, `500`

---
### `GET` `/.well-known/webauthn`
- **Description** : webauthn_well_known
- **Sécurité** : 🌐 Public
- **Tags** : `auth`
- **Codes Retour** : `200`, `500`

---
### `GET` `/auth/accounts`
- **Description** : get_accounts
- **Sécurité** : 🌐 Public
- **Tags** : `auth`
- **Codes Retour** : `200`

---
### `POST` `/auth/challenge/identifier`
- **Description** : challenge_identifier
- **Sécurité** : 🌐 Public
- **Tags** : `auth`
- **Codes Retour** : `200`, `403`

---
### `POST` `/auth/challenge/mfa`
- **Description** : challenge_mfa
- **Sécurité** : 🌐 Public
- **Tags** : `auth`
- **Codes Retour** : `200`, `401`

---
### `GET` `/auth/challenge/pow`
- **Description** : challenge_pow
- **Sécurité** : 🌐 Public
- **Tags** : `auth`
- **Codes Retour** : `200`

---
### `POST` `/auth/challenge/pwd`
- **Description** : challenge_pwd
- **Sécurité** : 🌐 Public
- **Tags** : `auth`
- **Codes Retour** : `200`, `202`, `401`, `403`

---
### `POST` `/auth/challenge/webauthn/discoverable/finish`
- **Description** : challenge_webauthn_discoverable_finish
- **Sécurité** : 🌐 Public
- **Tags** : `auth`
- **Codes Retour** : `200`, `202`, `401`

---
### `POST` `/auth/challenge/webauthn/discoverable/start`
- **Description** : challenge_webauthn_discoverable_start
- **Sécurité** : 🌐 Public
- **Tags** : `auth`
- **Codes Retour** : `200`, `401`

---
### `POST` `/auth/challenge/webauthn/start`
- **Description** : challenge_webauthn_start
- **Sécurité** : 🌐 Public
- **Tags** : `auth`
- **Codes Retour** : `200`, `401`

---
### `DELETE` `/auth/devices/{deviceId}`
- **Description** : revoke_device
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `auth`
- **Codes Retour** : `200`, `401`, `404`

---
### `POST` `/auth/devices/{deviceId}/trust`
- **Description** : trust_device
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `auth`
- **Codes Retour** : `200`, `401`, `404`

---
### `POST` `/auth/logout`
- **Description** : logout
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `auth`
- **Codes Retour** : `200`, `401`

---
### `GET` `/auth/me/emails`
- **Description** : me_emails_get
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `auth`
- **Codes Retour** : `200`, `401`

---
### `POST` `/auth/me/emails`
- **Description** : me_emails_post
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `auth`
- **Codes Retour** : `200`, `400`, `401`, `409`

---
### `DELETE` `/auth/me/emails/{emailId}`
- **Description** : me_email_delete
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `auth`
- **Codes Retour** : `200`, `401`, `403`, `404`

---
### `POST` `/auth/me/emails/{emailId}/promote`
- **Description** : me_email_promote
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `auth`
- **Codes Retour** : `200`, `401`, `403`, `404`

---
### `POST` `/auth/me/emails/{emailId}/resend-verification`
- **Description** : me_email_resend_verification
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `auth`
- **Codes Retour** : `200`, `400`, `401`, `404`

---
### `GET` `/auth/mfa/factors`
- **Description** : list_mfa_factors
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `auth`
- **Codes Retour** : `200`, `401`

---
### `DELETE` `/auth/mfa/factors/{factorId}`
- **Description** : remove_mfa_factor
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `auth`
- **Codes Retour** : `204`, `401`, `404`

---
### `POST` `/auth/mfa/recovery-codes`
- **Description** : generate_recovery_codes
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `auth`
- **Codes Retour** : `200`, `400`, `401`

---
### `POST` `/auth/mfa/totp/confirm`
- **Description** : confirm_totp_enrollment
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `auth`
- **Codes Retour** : `200`, `400`, `401`

---
### `POST` `/auth/mfa/totp/setup`
- **Description** : begin_totp_enrollment
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `auth`
- **Codes Retour** : `200`, `401`

---
### `POST` `/auth/mfa/webauthn/register/finish`
- **Description** : confirm_webauthn_enrollment
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `auth`
- **Codes Retour** : `200`, `400`, `401`

---
### `POST` `/auth/mfa/webauthn/register/start`
- **Description** : begin_webauthn_enrollment
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `auth`
- **Codes Retour** : `200`, `401`

---
### `POST` `/auth/mfa/webauthn/start`
- **Description** : webauthn_auth_start
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `auth`
- **Codes Retour** : `200`, `401`

---
### `POST` `/auth/password/change`
- **Description** : change_password
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `auth`
- **Codes Retour** : `200`, `400`, `401`, `429`

---
### `POST` `/auth/password/forgot`
- **Description** : forgot_password
- **Sécurité** : 🌐 Public
- **Tags** : `auth`
- **Codes Retour** : `200`, `400`, `429`

---
### `POST` `/auth/password/reset`
- **Description** : reset_password
- **Sécurité** : 🌐 Public
- **Tags** : `auth`
- **Codes Retour** : `200`, `400`, `429`

---
### `POST` `/auth/register`
- **Description** : register
- **Sécurité** : 🌐 Public
- **Tags** : `auth`
- **Codes Retour** : `200`, `400`, `409`, `429`

---
### `POST` `/auth/registration/availability`
- **Description** : registration_availability
- **Sécurité** : 🌐 Public
- **Tags** : `auth`
- **Codes Retour** : `200`, `400`, `429`

---
### `GET` `/auth/sessions`
- **Description** : list_sessions
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `auth`
- **Codes Retour** : `200`, `401`

---
### `POST` `/auth/sessions/revoke-all`
- **Description** : revoke_all_sessions
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `auth`
- **Codes Retour** : `200`, `401`

---
### `POST` `/auth/sessions/revoke-others`
- **Description** : revoke_all_other_sessions
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `auth`
- **Codes Retour** : `200`, `401`

---
### `DELETE` `/auth/sessions/{sessionId}`
- **Description** : revoke_session
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `auth`
- **Codes Retour** : `200`, `401`, `404`

---
### `POST` `/auth/sessions/{sessionId}/confirm`
- **Description** : confirm_high_risk_session
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `auth`
- **Codes Retour** : `200`, `401`, `404`

---
### `POST` `/auth/step-up`
- **Description** : step_up
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `auth`
- **Codes Retour** : `200`, `401`

---
### `POST` `/auth/step-up/email/request`
- **Description** : request_email_step_up
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `auth`
- **Codes Retour** : `200`, `400`, `401`, `429`

---
### `POST` `/auth/verify-email`
- **Description** : verify_email
- **Sécurité** : 🌐 Public
- **Tags** : `auth`
- **Codes Retour** : `200`, `400`, `429`

---
### `POST` `/auth/verify-email/change`
- **Description** : change_verify_email
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `auth`
- **Codes Retour** : `200`, `400`, `401`, `409`, `429`

---
### `POST` `/auth/verify-email/resend`
- **Description** : resend_verify_email
- **Sécurité** : 🌐 Public
- **Tags** : `auth`
- **Codes Retour** : `200`, `400`, `429`

---
### `POST` `/auth/workspaces/{workspaceId}/switch`
- **Description** : switch_workspace
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `auth`
- **Codes Retour** : `200`, `401`, `404`

---
### `POST` `/authz/decision`
- **Description** : decide
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `authz`
- **Codes Retour** : `200`, `400`, `401`, `500`

---
### `GET` `/oauth/authorize`
- **Description** : authorize
- **Sécurité** : 🌐 Public
- **Tags** : `oauth`
- **Codes Retour** : `303`, `400`, `401`, `404`, `500`

---
### `DELETE` `/oauth/client-policies/{policyId}`
- **Description** : delete_client_policy
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `oauth`
- **Codes Retour** : `200`, `401`, `404`, `500`

---
### `PATCH` `/oauth/client-policies/{policyId}`
- **Description** : update_client_policy
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `oauth`
- **Codes Retour** : `200`, `400`, `401`, `404`, `500`

---
### `GET` `/oauth/clients`
- **Description** : list_clients
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `oauth`
- **Codes Retour** : `200`, `401`, `500`

---
### `POST` `/oauth/clients`
- **Description** : create_client
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `oauth`
- **Codes Retour** : `200`, `400`, `401`, `500`

---
### `DELETE` `/oauth/clients/{clientId}`
- **Description** : revoke_client
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `oauth`
- **Codes Retour** : `200`, `400`, `401`, `404`, `500`

---
### `GET` `/oauth/clients/{clientId}/keys`
- **Description** : list_client_keys
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `oauth`
- **Codes Retour** : `200`, `401`, `404`

---
### `POST` `/oauth/clients/{clientId}/keys`
- **Description** : rotate_client_key
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `oauth`
- **Codes Retour** : `200`, `400`, `401`, `404`

---
### `DELETE` `/oauth/clients/{clientId}/keys/{keyId}`
- **Description** : revoke_client_key
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `oauth`
- **Codes Retour** : `200`, `401`, `404`, `409`

---
### `GET` `/oauth/clients/{clientId}/policies`
- **Description** : list_client_policies
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `oauth`
- **Codes Retour** : `200`, `401`, `500`

---
### `POST` `/oauth/clients/{clientId}/policies`
- **Description** : create_client_policy
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `oauth`
- **Codes Retour** : `200`, `400`, `401`, `500`

---
### `POST` `/oauth/device/approve`
- **Description** : device_approve
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `oauth`
- **Codes Retour** : `200`, `400`, `401`, `500`

---
### `POST` `/oauth/device/authorize`
- **Description** : device_authorize
- **Sécurité** : 🌐 Public
- **Tags** : `oauth`
- **Codes Retour** : `200`, `400`, `500`

---
### `POST` `/oauth/device/deny`
- **Description** : device_deny
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `oauth`
- **Codes Retour** : `200`, `400`, `401`, `500`

---
### `POST` `/oauth/device/verify`
- **Description** : device_verify
- **Sécurité** : 🌐 Public
- **Tags** : `oauth`
- **Codes Retour** : `200`, `400`, `500`

---
### `POST` `/oauth/introspect`
- **Description** : introspect
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `oauth`
- **Codes Retour** : `200`, `401`, `500`

---
### `POST` `/oauth/revoke`
- **Description** : revoke
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `oauth`
- **Codes Retour** : `200`, `400`, `401`

---
### `POST` `/oauth/token`
- **Description** : token
- **Sécurité** : 🌐 Public
- **Tags** : `oauth`
- **Codes Retour** : `200`, `400`, `401`, `500`

---
### `GET` `/oauth/userinfo`
- **Description** : userinfo
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `oauth`
- **Codes Retour** : `200`, `401`, `500`

---
### `GET` `/workspaces/{workspaceId}/security-events`
- **Description** : list_security_events
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `security`
- **Codes Retour** : `200`, `400`, `401`, `500`

---
### `GET` `/workspaces/{workspaceId}/security-events/export`
- **Description** : export_security_events
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `security`
- **Codes Retour** : `200`, `401`, `500`

---
