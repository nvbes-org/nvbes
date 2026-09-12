---
title: Catalogue des Services & Applications
description: Inventaire auto-généré des runtimes actifs et des applications archivées du monorepo nvbes.
---

> Ce document est généré automatiquement de manière déterministe à partir de l'état réel du monorepo.
> Dernière mise à jour : `2026-09-12T15:27:48.025Z`

## Vue d'ensemble

Le runtime actif contient **6 applications et services**. Les projets sous `archive/` sont exclus de ce total et ne constituent pas une roadmap produit.

| Application / Service             | Domaine               | Scope      | Type                   | Emplacement                        |
| :-------------------------------- | :-------------------- | :--------- | :--------------------- | :--------------------------------- |
| **`account-service`**             | `account`             | `platform` | ⚙️ Backend (Rust/Axum) | `apps/account-service`             |
| **`billing-service`**             | `billing`             | `platform` | ⚙️ Backend (Rust/Axum) | `apps/billing-service`             |
| **`email-worker`**                | `email`               | `platform` | ⚙️ Backend (Rust/Axum) | `apps/email-worker`                |
| **`identity-service`**            | `identity`            | `platform` | ⚙️ Backend (Rust/Axum) | `apps/identity-service`            |
| **`platform-operations-service`** | `platform-operations` | `platform` | ⚙️ Backend (Rust/Axum) | `apps/platform-operations-service` |
| **`trust-risk-service`**          | `trust-risk`          | `platform` | ⚙️ Backend (Rust/Axum) | `apps/trust-risk-service`          |

## Inventaire archivé

Les **19 projets** ci-dessous sont conservés comme historique. Ils ne sont ni déployables ni actifs sans une nouvelle décision produit et FinOps.

| Projet archivé       | Domaine      | Emplacement                         |
| :------------------- | :----------- | :---------------------------------- |
| `account-service`    | `account`    | `archive/apps/account-service-next` |
| `account-web`        | `account`    | `archive/apps/account-web`          |
| `account-worker`     | `account`    | `archive/apps/account-worker`       |
| `backoffice-service` | `backoffice` | `archive/apps/backoffice-service`   |
| `backoffice-web`     | `backoffice` | `archive/apps/backoffice-web`       |
| `billing-service`    | `billing`    | `archive/apps/billing-service`      |
| `billing-worker`     | `billing`    | `archive/apps/billing-worker`       |
| `cloud-service`      | `cloud`      | `archive/apps/cloud-service`        |
| `cloud-web`          | `cloud`      | `archive/apps/cloud-web`            |
| `cloud-worker`       | `cloud`      | `archive/apps/cloud-worker`         |
| `console-web`        | `developer`  | `archive/apps/console-web`          |
| `developer-service`  | `developer`  | `archive/apps/developer-service`    |
| `docs`               | `developer`  | `archive/apps/docs`                 |
| `enterprise-service` | `enterprise` | `archive/apps/enterprise-service`   |
| `enterprise-web`     | `enterprise` | `archive/apps/enterprise-web`       |
| `gateway-cloud`      | `cloud`      | `archive/apps/gateway-cloud`        |
| `identity-service`   | `identity`   | `archive/apps/identity-service`     |
| `identity-web`       | `identity`   | `archive/apps/identity-web`         |
| `identity-worker`    | `identity`   | `archive/apps/identity-worker`      |

## Détail des Services Backend

### `nvbes-account-service`

- **Chemin** : `apps/account-service`
- **Version** : `10.3.0`
- **Dépendances internes** : `nvbes-observability`

### `nvbes-billing-service`

- **Chemin** : `apps/billing-service`
- **Version** : `0.1.0`
- **Dépendances internes** : `nvbes-billing`, `nvbes-core`, `nvbes-observability`

### `nvbes-email-worker`

- **Chemin** : `apps/email-worker`
- **Version** : `=1.102.0`
- **Dépendances internes** : `nvbes-core`, `nvbes-email`, `nvbes-email-scaleway`, `nvbes-observability`

### `nvbes-identity-service`

- **Chemin** : `apps/identity-service`
- **Version** : `10.3.0`
- **Dépendances internes** : `nvbes-core`, `nvbes-email`, `nvbes-observability`

### `nvbes-trust-risk-service`

- **Chemin** : `apps/trust-risk-service`
- **Version** : `0.1.0`
- **Dépendances internes** : `nvbes-core`, `nvbes-observability`, `nvbes-trust-risk`, `opentelemetry-otlp`
