---
title: API nvbes Account Service
description: Contrats OpenAPI, routes, paramètres et schémas pour nvbes Account Service (0.1.0).
---

> Source : `apps/account-service-next/openapi.json` • Version : `0.1.0` • Endpoints : **20**

## Endpoints Disponibles

### `GET` `/api/v1/closure`
- **Description** : getAccountClosure
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `closure`
- **Codes Retour** : `200`, `401`, `403`, `404`

---
### `POST` `/api/v1/closure`
- **Description** : requestAccountClosure
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `closure`
- **Codes Retour** : `202`, `401`, `403`

---
### `GET` `/api/v1/consents`
- **Description** : listAccountConsents
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `privacy`
- **Codes Retour** : `200`, `400`, `401`, `403`

---
### `POST` `/api/v1/consents`
- **Description** : grantAccountConsent
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `privacy`
- **Codes Retour** : `200`, `400`, `401`, `403`

---
### `DELETE` `/api/v1/consents`
- **Description** : revokeAccountConsent
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `privacy`
- **Codes Retour** : `204`, `400`, `401`, `403`

---
### `GET` `/api/v1/notifications`
- **Description** : getAccountNotifications
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `settings`
- **Codes Retour** : `200`, `401`, `403`

---
### `PUT` `/api/v1/notifications`
- **Description** : updateAccountNotifications
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `settings`
- **Codes Retour** : `200`, `400`, `401`, `403`

---
### `GET` `/api/v1/preferences`
- **Description** : getAccountPreferences
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `settings`
- **Codes Retour** : `200`, `401`, `403`

---
### `PUT` `/api/v1/preferences`
- **Description** : updateAccountPreferences
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `settings`
- **Codes Retour** : `200`, `400`, `401`, `403`

---
### `POST` `/api/v1/privacy/exports`
- **Description** : requestAccountPrivacyExport
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `privacy`
- **Codes Retour** : `202`, `401`, `403`

---
### `GET` `/api/v1/privacy/exports/latest`
- **Description** : getLatestAccountPrivacyExport
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `privacy`
- **Codes Retour** : `200`, `401`, `403`, `404`

---
### `GET` `/api/v1/privacy/exports/{exportId}/document`
- **Description** : downloadAccountPrivacyExport
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `privacy`
- **Codes Retour** : `200`, `401`, `403`, `404`

---
### `GET` `/api/v1/privacy/gpc`
- **Description** : get_gpc_status
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `privacy`
- **Codes Retour** : `200`, `401`, `403`

---
### `GET` `/api/v1/profile`
- **Description** : getAccountProfile
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `profile`
- **Codes Retour** : `200`, `401`, `403`

---
### `PUT` `/api/v1/profile`
- **Description** : updateAccountProfile
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `profile`
- **Codes Retour** : `200`, `400`, `401`, `403`, `409`

---
### `GET` `/api/v1/profile/avatar`
- **Description** : downloadAccountAvatar
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `profile`
- **Codes Retour** : `307`, `401`, `403`, `404`

---
### `POST` `/api/v1/profile/avatar`
- **Description** : prepareAccountAvatarUpload
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `profile`
- **Codes Retour** : `200`, `400`, `401`, `403`

---
### `DELETE` `/api/v1/profile/avatar`
- **Description** : deleteAccountAvatar
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `profile`
- **Codes Retour** : `200`, `401`, `403`

---
### `GET` `/api/v1/security/sessions`
- **Description** : list_sessions
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `security`
- **Codes Retour** : `200`

---
### `DELETE` `/api/v1/security/sessions/{sessionId}`
- **Description** : revoke_session
- **Sécurité** : 🔒 Sécurisé
- **Tags** : `security`
- **Codes Retour** : `200`

---
