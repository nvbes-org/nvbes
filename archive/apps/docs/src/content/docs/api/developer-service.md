---
title: API nvbes Developer Service
description: Contrats OpenAPI, routes, paramètres et schémas pour nvbes Developer Service (0.1.0).
---

> Source : `apps/developer-service/openapi.json` • Version : `0.1.0` • Endpoints : **38**

## Endpoints Disponibles

### `GET` `/developer/apps`
- **Description** : list_apps
- **Sécurité** : 🌐 Public
- **Tags** : `developer`
- **Codes Retour** : `200`

---
### `POST` `/developer/apps`
- **Description** : create_app
- **Sécurité** : 🌐 Public
- **Tags** : `developer`
- **Codes Retour** : `200`

---
### `GET` `/developer/apps/{clientId}`
- **Description** : get_app
- **Sécurité** : 🌐 Public
- **Tags** : `developer`
- **Codes Retour** : `200`

---
### `DELETE` `/developer/apps/{clientId}`
- **Description** : revoke_app
- **Sécurité** : 🌐 Public
- **Tags** : `developer`
- **Codes Retour** : `204`

---
### `PATCH` `/developer/apps/{clientId}/redirects`
- **Description** : update_redirects
- **Sécurité** : 🌐 Public
- **Tags** : `developer`
- **Codes Retour** : `200`

---
### `GET` `/developer/console/context`
- **Description** : context
- **Sécurité** : 🌐 Public
- **Tags** : `developer-console`
- **Codes Retour** : `200`

---
### `GET` `/developer/console/health-checks`
- **Description** : list_health_checks
- **Sécurité** : 🌐 Public
- **Tags** : `developer-console`
- **Codes Retour** : `200`

---
### `POST` `/developer/console/health-checks`
- **Description** : run_health_checks
- **Sécurité** : 🌐 Public
- **Tags** : `developer-console`
- **Codes Retour** : `200`

---
### `GET` `/developer/console/logs`
- **Description** : list_api_logs
- **Sécurité** : 🌐 Public
- **Tags** : `developer-console`
- **Codes Retour** : `200`

---
### `GET` `/developer/console/marketplace/apps`
- **Description** : list_marketplace_apps
- **Sécurité** : 🌐 Public
- **Tags** : `developer-console`
- **Codes Retour** : `200`

---
### `POST` `/developer/console/marketplace/apps/{clientId}/review`
- **Description** : review_marketplace_app
- **Sécurité** : 🌐 Public
- **Tags** : `developer-console`
- **Codes Retour** : `200`

---
### `POST` `/developer/console/marketplace/apps/{clientId}/submit`
- **Description** : submit_marketplace_app
- **Sécurité** : 🌐 Public
- **Tags** : `developer-console`
- **Codes Retour** : `200`

---
### `GET` `/developer/console/oauth-clients`
- **Description** : list_oauth_clients
- **Sécurité** : 🌐 Public
- **Tags** : `developer-console`
- **Codes Retour** : `200`

---
### `GET` `/developer/console/oauth-clients/{clientId}/consent-screen`
- **Description** : get_consent_screen
- **Sécurité** : 🌐 Public
- **Tags** : `developer-console`
- **Codes Retour** : `200`

---
### `PUT` `/developer/console/oauth-clients/{clientId}/consent-screen`
- **Description** : upsert_consent_screen
- **Sécurité** : 🌐 Public
- **Tags** : `developer-console`
- **Codes Retour** : `200`

---
### `GET` `/developer/console/oauth-clients/{clientId}/secrets`
- **Description** : list_secret_versions
- **Sécurité** : 🌐 Public
- **Tags** : `developer-console`
- **Codes Retour** : `200`

---
### `POST` `/developer/console/oauth-clients/{clientId}/secrets/rotation`
- **Description** : rotate_secret
- **Sécurité** : 🌐 Public
- **Tags** : `developer-console`
- **Codes Retour** : `200`

---
### `POST` `/developer/console/oauth-clients/{clientId}/secrets/{versionId}/revoke`
- **Description** : revoke_secret_version
- **Sécurité** : 🌐 Public
- **Tags** : `developer-console`
- **Codes Retour** : `200`

---
### `GET` `/developer/console/overview`
- **Description** : overview
- **Sécurité** : 🌐 Public
- **Tags** : `developer-console`
- **Codes Retour** : `200`

---
### `GET` `/developer/console/sandbox`
- **Description** : get_sandbox
- **Sécurité** : 🌐 Public
- **Tags** : `developer-console`
- **Codes Retour** : `200`

---
### `PUT` `/developer/console/sandbox`
- **Description** : upsert_sandbox
- **Sécurité** : 🌐 Public
- **Tags** : `developer-console`
- **Codes Retour** : `200`

---
### `POST` `/developer/console/sandbox/reset`
- **Description** : reset_sandbox
- **Sécurité** : 🌐 Public
- **Tags** : `developer-console`
- **Codes Retour** : `200`

---
### `GET` `/developer/console/scopes`
- **Description** : list_scopes
- **Sécurité** : 🌐 Public
- **Tags** : `developer-console`
- **Codes Retour** : `200`

---
### `POST` `/developer/console/scopes`
- **Description** : create_scope
- **Sécurité** : 🌐 Public
- **Tags** : `developer-console`
- **Codes Retour** : `200`

---
### `PUT` `/developer/console/scopes/{scopeKey}`
- **Description** : update_scope
- **Sécurité** : 🌐 Public
- **Tags** : `developer-console`
- **Codes Retour** : `200`

---
### `DELETE` `/developer/console/scopes/{scopeKey}`
- **Description** : delete_scope
- **Sécurité** : 🌐 Public
- **Tags** : `developer-console`
- **Codes Retour** : `204`

---
### `GET` `/developer/console/service-accounts`
- **Description** : list_service_accounts
- **Sécurité** : 🌐 Public
- **Tags** : `developer-console`
- **Codes Retour** : `200`

---
### `POST` `/developer/console/tokens/debug`
- **Description** : debug_token
- **Sécurité** : 🌐 Public
- **Tags** : `developer-console`
- **Codes Retour** : `200`

---
### `GET` `/developer/console/webhooks`
- **Description** : list_console_webhooks
- **Sécurité** : 🌐 Public
- **Tags** : `developer-console`
- **Codes Retour** : `200`

---
### `POST` `/developer/console/webhooks/deliveries/{deliveryId}/replay`
- **Description** : replay_delivery
- **Sécurité** : 🌐 Public
- **Tags** : `developer-console`
- **Codes Retour** : `200`

---
### `GET` `/developer/console/webhooks/{endpointId}/deliveries`
- **Description** : list_deliveries
- **Sécurité** : 🌐 Public
- **Tags** : `developer-console`
- **Codes Retour** : `200`

---
### `GET` `/developer/logs`
- **Description** : list_logs
- **Sécurité** : 🌐 Public
- **Tags** : `developer`
- **Codes Retour** : `200`

---
### `GET` `/developer/me`
- **Description** : me
- **Sécurité** : 🌐 Public
- **Tags** : `developer`
- **Codes Retour** : `200`

---
### `POST` `/developer/oauth/playground/exchange`
- **Description** : exchange_playground_code
- **Sécurité** : 🌐 Public
- **Tags** : `developer`
- **Codes Retour** : `200`

---
### `POST` `/developer/tokens/inspect`
- **Description** : inspect_token
- **Sécurité** : 🌐 Public
- **Tags** : `developer`
- **Codes Retour** : `200`

---
### `GET` `/developer/webhooks`
- **Description** : list_portal_webhooks
- **Sécurité** : 🌐 Public
- **Tags** : `developer`
- **Codes Retour** : `200`

---
### `POST` `/developer/webhooks`
- **Description** : create_portal_webhook
- **Sécurité** : 🌐 Public
- **Tags** : `developer`
- **Codes Retour** : `200`

---
### `DELETE` `/developer/webhooks/{endpointId}`
- **Description** : delete_portal_webhook
- **Sécurité** : 🌐 Public
- **Tags** : `developer`
- **Codes Retour** : `204`

---
