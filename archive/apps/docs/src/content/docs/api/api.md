---
title: API nvbes Cloud API
description: Contrats OpenAPI, routes, paramètres et schémas pour nvbes Cloud API (0.1.0).
---

> Source : `docs/api/openapi/drive-public-v1.openapi.json` • Version : `0.1.0` • Endpoints : **17**

## Endpoints Disponibles

### `GET` `/v1/me`
- **Description** : me
- **Sécurité** : 🌐 Public
- **Tags** : `public-api`
- **Codes Retour** : `200`, `401`, `500`

---
### `GET` `/v1/workspaces`
- **Description** : list_workspaces
- **Sécurité** : 🌐 Public
- **Tags** : `public-api`
- **Codes Retour** : `200`, `401`, `500`

---
### `GET` `/v1/workspaces/{workspaceId}/audit-events`
- **Description** : list_audit_events
- **Sécurité** : 🌐 Public
- **Tags** : `public-api`
- **Codes Retour** : `200`, `401`, `500`

---
### `POST` `/v1/workspaces/{workspaceId}/folders`
- **Description** : create_folder
- **Sécurité** : 🌐 Public
- **Tags** : `public-api`
- **Codes Retour** : `200`, `400`, `401`, `500`

---
### `GET` `/v1/workspaces/{workspaceId}/objects`
- **Description** : list_objects
- **Sécurité** : 🌐 Public
- **Tags** : `public-api`
- **Codes Retour** : `200`, `401`, `500`

---
### `PATCH` `/v1/workspaces/{workspaceId}/objects/{objectId}`
- **Description** : rename_object
- **Sécurité** : 🌐 Public
- **Tags** : `public-api`
- **Codes Retour** : `200`, `400`, `401`, `404`, `500`

---
### `POST` `/v1/workspaces/{workspaceId}/objects/{objectId}/download-url`
- **Description** : create_download_url
- **Sécurité** : 🌐 Public
- **Tags** : `public-api`
- **Codes Retour** : `200`, `401`, `404`, `500`

---
### `POST` `/v1/workspaces/{workspaceId}/objects/{objectId}/move`
- **Description** : move_object
- **Sécurité** : 🌐 Public
- **Tags** : `public-api`
- **Codes Retour** : `200`, `400`, `401`, `404`, `500`

---
### `POST` `/v1/workspaces/{workspaceId}/objects/{objectId}/share-links`
- **Description** : create_share_link
- **Sécurité** : 🌐 Public
- **Tags** : `public-api`
- **Codes Retour** : `200`, `400`, `401`, `404`, `500`

---
### `POST` `/v1/workspaces/{workspaceId}/objects/{objectId}/trash`
- **Description** : trash_object
- **Sécurité** : 🌐 Public
- **Tags** : `public-api`
- **Codes Retour** : `200`, `401`, `404`, `500`

---
### `GET` `/v1/workspaces/{workspaceId}/quota`
- **Description** : get_quota
- **Sécurité** : 🌐 Public
- **Tags** : `public-api`
- **Codes Retour** : `200`, `401`, `500`

---
### `GET` `/v1/workspaces/{workspaceId}/share-links`
- **Description** : list_share_links
- **Sécurité** : 🌐 Public
- **Tags** : `public-api`
- **Codes Retour** : `200`, `401`, `500`

---
### `DELETE` `/v1/workspaces/{workspaceId}/share-links/{shareLinkId}`
- **Description** : revoke_share_link
- **Sécurité** : 🌐 Public
- **Tags** : `public-api`
- **Codes Retour** : `200`, `401`, `404`, `500`

---
### `PATCH` `/v1/workspaces/{workspaceId}/share-links/{shareLinkId}`
- **Description** : update_share_link
- **Sécurité** : 🌐 Public
- **Tags** : `public-api`
- **Codes Retour** : `200`, `400`, `401`, `404`, `500`

---
### `POST` `/v1/workspaces/{workspaceId}/uploads`
- **Description** : create_upload
- **Sécurité** : 🌐 Public
- **Tags** : `public-api`
- **Codes Retour** : `200`, `400`, `401`, `500`

---
### `POST` `/v1/workspaces/{workspaceId}/uploads/{uploadId}/cancel`
- **Description** : cancel_upload
- **Sécurité** : 🌐 Public
- **Tags** : `public-api`
- **Codes Retour** : `200`, `401`, `404`, `500`

---
### `POST` `/v1/workspaces/{workspaceId}/uploads/{uploadId}/complete`
- **Description** : complete_upload
- **Sécurité** : 🌐 Public
- **Tags** : `public-api`
- **Codes Retour** : `200`, `400`, `401`, `404`, `500`

---
