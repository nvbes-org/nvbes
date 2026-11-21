---
title: Catalogue des Services & Applications
description: Inventaire auto-généré des runtimes actifs et des applications archivées du monorepo nvbes.
---

> Ce document est généré automatiquement de manière déterministe à partir de l'état réel du monorepo.
> Dernière mise à jour : `2026-11-21T18:27:12.321Z`

## Vue d'ensemble

Le runtime actif contient **4 applications et services**. Les projets sous `archive/` sont exclus de ce total et ne constituent pas une roadmap produit.

| Application / Service | Domaine | Scope | Type | Emplacement |
| :--- | :--- | :--- | :--- | :--- |
| **`api`** | `api` | `platform` | ⚙️ Backend (Rust/Axum) | `apps/api` |
| **`worker`** | `worker` | `platform` | ⚙️ Backend (Rust/Axum) | `apps/worker` |
| **`account-web`** | `account` | `platform` | 💻 Frontend (React/Vite) | `apps/account-web` |
| **`identity-web`** | `identity` | `platform` | 💻 Frontend (React/Vite) | `apps/identity-web` |

## Inventaire archivé

Les **23 projets** ci-dessous sont conservés comme historique. Ils ne sont ni déployables ni actifs sans une nouvelle décision produit et FinOps.

| Projet archivé | Domaine | Emplacement |
| :--- | :--- | :--- |
| `account-service` | `account` | `archive/apps/account-service` |
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
| `email-worker` | `email` | `archive/apps/email-worker` |
| `enterprise-service` | `enterprise` | `archive/apps/enterprise-service` |
| `enterprise-web` | `enterprise` | `archive/apps/enterprise-web` |
| `gateway-cloud` | `cloud` | `archive/apps/gateway-cloud` |
| `identity-service` | `identity` | `archive/apps/identity-service` |
| `identity-web` | `identity` | `archive/apps/identity-web` |
| `identity-worker` | `identity` | `archive/apps/identity-worker` |
| `platform-operations-service` | `platform-operations` | `archive/apps/platform-operations-service` |
| `trust-risk-service` | `trust-risk` | `archive/apps/trust-risk-service` |

## Détail des Services Backend

### `nvbes-api`
- **Chemin** : `apps/api`
- **Version** : `0.1.0`
- **Dépendances internes** : `nvbes-account-service`, `nvbes-billing-service`, `nvbes-consent`, `nvbes-core`, `nvbes-identity-service`, `nvbes-observability`, `tower-http`

### `nvbes-worker`
- **Chemin** : `apps/worker`
- **Version** : `0.1.0`
- **Dépendances internes** : `nvbes-account-service`, `nvbes-billing-service`, `nvbes-core`, `nvbes-email`, `nvbes-email-scaleway`, `nvbes-observability`
