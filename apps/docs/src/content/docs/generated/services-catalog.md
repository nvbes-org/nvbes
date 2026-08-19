---
title: Catalogue des Services & Applications
description: Inventaire auto-généré de toutes les applications et microservices du monorepo nvbes.
---

> Ce document est généré automatiquement de manière déterministe à partir de l'état réel du monorepo.
> Dernière mise à jour : `2026-08-19T22:51:12.120Z`

## Vue d'ensemble

Le monorepo contient **20 applications et services** (backends Axum, frontends React/Vite, workers asynchrones).

| Application / Service | Domaine | Scope | Type | Emplacement |
| :--- | :--- | :--- | :--- | :--- |
| **`account-service`** | `account` | `oss` | ⚙️ Backend (Rust/Axum) | `apps/account-service-next` |
| **`account-web`** | `account` | `oss` | 💻 Frontend (React/Vite) | `apps/account-web` |
| **`account-worker`** | `account` | `oss` | ⚙️ Backend (Rust/Axum) | `apps/account-worker` |
| **`backoffice-service`** | `backoffice` | `internal` | ⚙️ Backend (Rust/Axum) | `apps/backoffice-service` |
| **`backoffice-web`** | `backoffice` | `internal` | 💻 Frontend (React/Vite) | `apps/backoffice-web` |
| **`billing-service`** | `billing` | `cloud` | ⚙️ Backend (Rust/Axum) | `apps/billing-service` |
| **`billing-worker`** | `billing` | `cloud` | ⚙️ Backend (Rust/Axum) | `apps/billing-worker` |
| **`cloud-service`** | `cloud` | `oss` | ⚙️ Backend (Rust/Axum) | `apps/cloud-service` |
| **`cloud-web`** | `cloud` | `oss` | 💻 Frontend (React/Vite) | `apps/cloud-web` |
| **`cloud-worker`** | `cloud` | `oss` | ⚙️ Backend (Rust/Axum) | `apps/cloud-worker` |
| **`console-web`** | `developer` | `oss` | 💻 Frontend (React/Vite) | `apps/console-web` |
| **`developer-service`** | `developer` | `oss` | ⚙️ Backend (Rust/Axum) | `apps/developer-service` |
| **`docs`** | `developer` | `internal` | 💻 Frontend (React/Vite) | `apps/docs` |
| **`email-worker`** | `email` | `cloud` | ⚙️ Backend (Rust/Axum) | `apps/email-worker` |
| **`enterprise-service`** | `enterprise` | `oss` | ⚙️ Backend (Rust/Axum) | `apps/enterprise-service` |
| **`enterprise-web`** | `enterprise` | `oss` | 💻 Frontend (React/Vite) | `apps/enterprise-web` |
| **`gateway-cloud`** | `cloud` | `oss` | 💻 Frontend (React/Vite) | `apps/gateway-cloud` |
| **`identity-service`** | `identity` | `cloud` | ⚙️ Backend (Rust/Axum) | `apps/identity-service` |
| **`identity-web`** | `identity` | `oss` | 💻 Frontend (React/Vite) | `apps/identity-web` |
| **`identity-worker`** | `identity` | `oss` | ⚙️ Backend (Rust/Axum) | `apps/identity-worker` |

## Détail des Services Backend

### `nvbes-account-service`
- **Chemin** : `apps/account-service-next`
- **Version** : `0.1.0`
- **Dépendances internes** : `nvbes-identity-sdk`, `nvbes-product-account`, `nvbes-storage`, `sqlx`, `tower-http`, `tracing-subscriber`

### `nvbes-account-worker`
- **Chemin** : `apps/account-worker`
- **Version** : `0.1.0`
- **Dépendances internes** : `nvbes-account-service`, `nvbes-core`, `nvbes-product-account`, `nvbes-storage`, `sqlx`

### `nvbes-backoffice-service`
- **Chemin** : `apps/backoffice-service`
- **Version** : `0.1.0`
- **Dépendances internes** : `nvbes-billing`, `nvbes-core`, `nvbes-email`, `nvbes-observability`, `tower-http`, `tracing-subscriber`

### `nvbes-billing-service`
- **Chemin** : `apps/billing-service`
- **Version** : `0.1.0`
- **Dépendances internes** : `nvbes-billing`, `nvbes-analytics-posthog`, `nvbes-core`, `nvbes-observability`, `nvbes-product-analytics`, `nvbes-product-account`, `nvbes-redis`, `tower-http`

### `nvbes-billing-worker`
- **Chemin** : `apps/billing-worker`
- **Version** : `0.1.0`
- **Dépendances internes** : `nvbes-billing`, `nvbes-billing-service`, `nvbes-core`, `nvbes-email`, `nvbes-observability`, `nvbes-analytics-posthog`, `nvbes-product-analytics`, `nvbes-redis`

### `nvbes-cloud-service`
- **Chemin** : `apps/cloud-service`
- **Version** : `10.3.0`
- **Dépendances internes** : `nvbes-core`, `nvbes-audit`, `nvbes-redis`, `nvbes-scan`, `nvbes-storage`, `nvbes-tenancy`, `nvbes-observability`, `nvbes-analytics-posthog`, `nvbes-product-analytics`, `nvbes-product-account`, `nvbes-product-cloud`, `nvbes-identity-sdk`, `nvbes-region`, `tower-http`, `tracing-subscriber`, `utoipa`, `utoipa-swagger-ui`

### `nvbes-cloud-worker`
- **Chemin** : `apps/cloud-worker`
- **Version** : `0.29`
- **Dépendances internes** : `nvbes-core`, `nvbes-redis`, `nvbes-observability`, `nvbes-product-cloud`, `nvbes-region`, `nvbes-storage`

### `nvbes-developer-service`
- **Chemin** : `apps/developer-service`
- **Version** : `10.3.0`
- **Dépendances internes** : `nvbes-core`, `nvbes-observability`, `nvbes-product-identity`, `tower-http`, `tracing-subscriber`

### `nvbes-email-worker`
- **Chemin** : `apps/email-worker`
- **Version** : `=1.90.0`
- **Dépendances internes** : `nvbes-core`, `nvbes-email`, `nvbes-email-scaleway`, `nvbes-observability`

### `nvbes-enterprise-service`
- **Chemin** : `apps/enterprise-service`
- **Version** : `0.1.0`
- **Dépendances internes** : `nvbes-core`, `nvbes-observability`, `tracing-subscriber`

### `nvbes-gateway-cloud`
- **Chemin** : `apps/gateway-cloud`
- **Version** : `0.1.0`
- **Dépendances internes** : `nvbes-core`, `nvbes-observability`, `tower-http`

### `nvbes-identity-service`
- **Chemin** : `apps/identity-service`
- **Version** : `10.3.0`
- **Dépendances internes** : `nvbes-core`, `nvbes-dpop`, `nvbes-redis`, `nvbes-audit`, `nvbes-email`, `nvbes-region`, `nvbes-tenancy`, `nvbes-observability`, `nvbes-analytics-posthog`, `nvbes-product-analytics`, `nvbes-product-account`, `nvbes-product-cloud`, `nvbes-product-identity`, `nvbes-storage`, `sqlx`, `tower-http`, `tracing-subscriber`, `utoipa`, `utoipa-swagger-ui`, `webauthn-rs`

### `nvbes-identity-worker`
- **Chemin** : `apps/identity-worker`
- **Version** : `0.29`
- **Dépendances internes** : `nvbes-email`, `nvbes-core`, `nvbes-storage`, `nvbes-redis`, `nvbes-observability`, `nvbes-product-identity`, `nvbes-product-analytics`, `nvbes-region`, `tonic`, `tracing-subscriber`

