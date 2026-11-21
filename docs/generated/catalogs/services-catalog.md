---
title: Catalogue des Services & Applications
description: Inventaire auto-généré des runtimes actifs et des applications archivées du monorepo nvbes.
---

> Ce document est généré automatiquement de manière déterministe à partir de l'état réel du monorepo.
> Dernière mise à jour : `2026-11-21T13:52:36.832Z`

## Vue d'ensemble

Le runtime actif contient **9 applications et services**. Les projets sous `archive/` sont exclus de ce total et ne constituent pas une roadmap produit.

| Application / Service | Domaine | Scope | Type | Emplacement |
| :--- | :--- | :--- | :--- | :--- |
| **`account-service`** | `account` | `platform` | ⚙️ Backend (Rust/Axum) | `apps/account-service` |
| **`api`** | `api` | `platform` | 💻 Frontend (React/Vite) | `apps/api` |
| **`billing-service`** | `billing` | `platform` | ⚙️ Backend (Rust/Axum) | `apps/billing-service` |
| **`billing-worker`** | `billing` | `platform` | ⚙️ Backend (Rust/Axum) | `apps/billing-worker` |
| **`email-worker`** | `email` | `platform` | ⚙️ Backend (Rust/Axum) | `apps/email-worker` |
| **`identity-service`** | `identity` | `platform` | ⚙️ Backend (Rust/Axum) | `apps/identity-service` |
| **`platform-operations-service`** | `platform-operations` | `platform` | ⚙️ Backend (Rust/Axum) | `apps/platform-operations-service` |
| **`trust-risk-service`** | `trust-risk` | `platform` | ⚙️ Backend (Rust/Axum) | `apps/trust-risk-service` |
| **`worker`** | `worker` | `platform` | 💻 Frontend (React/Vite) | `apps/worker` |

## Inventaire archivé

Les **19 projets** ci-dessous sont conservés comme historique. Ils ne sont ni déployables ni actifs sans une nouvelle décision produit et FinOps.

| Projet archivé | Domaine | Emplacement |
| :--- | :--- | :--- |
| `account-service` | `account` | `archive/apps/account-service-next` |
| `account-web` | `account` | `archive/apps/account-web` |
| `account-worker` | `account` | `archive/apps/account-worker` |
| `backoffice-service` | `backoffice` | `archive/apps/backoffice-service` |
| `backoffice-web` | `backoffice` | `archive/apps/backoffice-web` |
| `billing-service` | `billing` | `archive/apps/billing-service` |
| `billing-worker` | `billing` | `archive/apps/billing-worker` |
| `cloud-service` | `cloud` | `archive/apps/cloud-service` |
| `cloud-web` | `cloud` | `archive/apps/cloud-web` |
| `cloud-worker` | `cloud` | `archive/apps/cloud-worker` |
| `console-web` | `developer` | `archive/apps/console-web` |
| `developer-service` | `developer` | `archive/apps/developer-service` |
| `docs` | `developer` | `archive/apps/docs` |
| `enterprise-service` | `enterprise` | `archive/apps/enterprise-service` |
| `enterprise-web` | `enterprise` | `archive/apps/enterprise-web` |
| `gateway-cloud` | `cloud` | `archive/apps/gateway-cloud` |
| `identity-service` | `identity` | `archive/apps/identity-service` |
| `identity-web` | `identity` | `archive/apps/identity-web` |
| `identity-worker` | `identity` | `archive/apps/identity-worker` |

## Détail des Services Backend

### `nvbes-account-service`
- **Chemin** : `apps/account-service`
- **Version** : `10.3.0`
- **Dépendances internes** : `nvbes-identity-sdk`, `nvbes-core`, `nvbes-dpop`, `nvbes-observability`

### `nvbes-api`
- **Chemin** : `apps/api`
- **Version** : `0.1.0`
- **Dépendances internes** : `nvbes-account-service`, `nvbes-billing-service`, `nvbes-consent`, `nvbes-core`, `nvbes-identity-service`, `nvbes-observability`, `tower-http`

### `nvbes-billing-service`
- **Chemin** : `apps/billing-service`
- **Version** : `10.3.0`
- **Dépendances internes** : `nvbes-billing`, `nvbes-core`, `nvbes-identity-sdk`, `nvbes-dpop`, `nvbes-email`, `nvbes-observability`

### `nvbes-billing-worker`
- **Chemin** : `apps/billing-worker`
- **Version** : `=1.112.0`
- **Dépendances internes** : `nvbes-billing`, `nvbes-core`, `nvbes-email`, `nvbes-observability`

### `nvbes-email-worker`
- **Chemin** : `apps/email-worker`
- **Version** : `=1.112.0`
- **Dépendances internes** : `nvbes-core`, `nvbes-email`, `nvbes-email-scaleway`, `nvbes-observability`

### `nvbes-identity-service`
- **Chemin** : `apps/identity-service`
- **Version** : `10.3.0`
- **Dépendances internes** : `nvbes-core`, `nvbes-dpop`, `nvbes-email`, `nvbes-observability`, `webauthn-rs`

### `nvbes-trust-risk-service`
- **Chemin** : `apps/trust-risk-service`
- **Version** : `0.1.0`
- **Dépendances internes** : `nvbes-core`, `nvbes-observability`, `nvbes-trust-risk`, `opentelemetry-otlp`

### `nvbes-worker`
- **Chemin** : `apps/worker`
- **Version** : `0.1.0`
- **Dépendances internes** : `nvbes-account-service`, `nvbes-billing-service`, `nvbes-core`, `nvbes-email`, `nvbes-email-scaleway`, `nvbes-observability`
