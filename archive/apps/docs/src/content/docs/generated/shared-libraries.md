---
title: Bibliothèques Partagées (Rust & TypeScript)
description: Répertoire exhaustif des packages et crates partagés au sein du workspace.
---

> Ce document est auto-généré à partir de l'analyse des workspaces Cargo et pnpm.

## 🦀 Bibliothèques Rust (`libs/rust/`)

Total : **22 crates partagées**

| Crate | Chemin | Version | Dépendances internes |
| :--- | :--- | :--- | :--- |
| **`nvbes-analytics-posthog`** | `libs/rust/adapters-cloud/analytics-posthog` | `0.1.0` | `nvbes-product-analytics` |
| **`nvbes-email-scaleway`** | `libs/rust/adapters-cloud/email-scaleway` | `0.1.0` | `nvbes-core`, `nvbes-email` |
| **`nvbes-audit`** | `libs/rust/audit` | `0.1.0` | - |
| **`nvbes-billing`** | `libs/rust/billing` | `0.1.0` | `nvbes-core`, `nvbes-audit`, `nvbes-redis`, `nvbes-region` |
| **`nvbes-core`** | `libs/rust/core` | `0.6` | `tokio`, `tower-http`, `nvbes-redis`, `utoipa` |
| **`nvbes-dpop`** | `libs/rust/dpop` | `10` | `nvbes-redis`, `serde` |
| **`nvbes-email`** | `libs/rust/email` | `0.23.41` | `nvbes-core`, `tonic` |
| **`nvbes-identity-sdk`** | `libs/rust/identity-sdk-backend` | `0.1.0` | `reqwest`, `serde`, `serde_json`, `urlencoding`, `thiserror`, `base64`, `sha2`, `rand`, `tokio` |
| **`nvbes-observability`** | `libs/rust/observability` | `0.1.0` | `nvbes-core`, `tracing-subscriber`, `opentelemetry`, `opentelemetry_sdk`, `opentelemetry-otlp`, `pyroscope`, `tracing-opentelemetry` |
| **`nvbes-platform`** | `libs/rust/platform` | `0.1.0` | - |
| **`nvbes-ports`** | `libs/rust/ports` | `0.1.0` | `nvbes-platform` |
| **`nvbes-product-analytics`** | `libs/rust/product-analytics` | `0.1.0` | - |
| **`nvbes-product-account`** | `libs/rust/products/account` | `0.1.0` | `nvbes-core`, `nvbes-redis` |
| **`nvbes-product-cloud`** | `libs/rust/products/cloud` | `0.1.0` | `nvbes-core`, `nvbes-storage` |
| **`nvbes-product-developer`** | `libs/rust/products/developer` | `0.1.0` | - |
| **`nvbes-product-enterprise`** | `libs/rust/products/enterprise` | `0.1.0` | - |
| **`nvbes-product-identity`** | `libs/rust/products/identity` | `0.1.0` | `nvbes-core`, `nvbes-email`, `nvbes-redis` |
| **`nvbes-redis`** | `libs/rust/redis` | `0.29` | - |
| **`nvbes-region`** | `libs/rust/region` | `0.1.0` | - |
| **`nvbes-scan`** | `libs/rust/scan` | `0.1.0` | - |
| **`nvbes-storage`** | `libs/rust/storage` | `1.8.16` | `nvbes-core` |
| **`nvbes-tenancy`** | `libs/rust/tenancy` | `0.1.0` | `nvbes-core` |

## 🟦 Bibliothèques TypeScript / SDKs (`libs/ts/`)

Total : **11 packages partagés**

| Package | Chemin | Version | Description / Rôle |
| :--- | :--- | :--- | :--- |
| **`@nvbes/account-client`** | `libs/ts/account-client` | `0.1.0` | Typed OAuth resource client for nvbes Account |
| **`@nvbes/backoffice-service-sdk-core`** | `libs/ts/backoffice-service-sdk-core` | `0.1.0` | OpenAPI specification and generated TypeScript types for nvbes Backoffice |
| **`@nvbes/billing-client`** | `libs/ts/billing-client` | `0.1.0` | Typed browser client for nvbes billing-service |
| **`@nvbes/email-ui`** | `libs/ts/email-ui` | `0.1.0` | React Email design system and transactional email templates for nvbes |
| **`@nvbes/http-client`** | `libs/ts/http-client` | `0.1.0` | Shared fetch transport with runtime DTO validation |
| **`@nvbes/identity-client`** | `libs/ts/identity-client` | `0.1.0` | Typed browser client for nvbes Account Service |
| **`@nvbes/identity-sdk`** | `libs/ts/identity-sdk` | `0.1.0` | SDK TypeScript pour intégrer nvbes Identity (OAuth2) |
| **`@nvbes/identity-sdk-core`** | `libs/ts/identity-sdk-core` | `0.1.0` | OpenAPI specification for nvbes Identity - cross-language SDK generation |
| **`@nvbes/identity-sdk-web`** | `libs/ts/identity-sdk-web` | `0.1.0` | nvbes Identity SDK for web browsers (secure cookies) |
| **`@nvbes/web-runtime`** | `libs/ts/web-runtime` | `0.1.0` | Shared frontend runtime for TanStack Query and client errors |
| **`@nvbes/web-ui`** | `libs/ts/web-ui` | `0.1.0` | Shared composite web UI for reusable account switching flows |
