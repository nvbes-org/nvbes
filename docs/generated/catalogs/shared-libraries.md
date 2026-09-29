---
title: Bibliothèques Partagées (Rust & TypeScript)
description: Répertoire exhaustif des packages et crates partagés au sein du workspace.
---

> Ce document est auto-généré à partir de l'analyse des workspaces Cargo et pnpm.

## 🦀 Bibliothèques Rust (`libs/rust/`)

Total : **10 crates partagées**

| Crate | Chemin | Version | Dépendances internes |
| :--- | :--- | :--- | :--- |
| **`nvbes-email-scaleway`** | `libs/rust/adapters/email-scaleway` | `0.1.0` | `nvbes-core`, `nvbes-email` |
| **`nvbes-audit`** | `libs/rust/audit` | `0.1.0` | - |
| **`nvbes-billing`** | `libs/rust/billing` | `0.23.41` | `nvbes-core`, `nvbes-audit`, `nvbes-region`, `tonic` |
| **`nvbes-core`** | `libs/rust/core` | `0.6` | `tokio`, `tower-http`, `utoipa` |
| **`nvbes-email`** | `libs/rust/email` | `0.23.41` | `nvbes-core`, `tonic` |
| **`nvbes-observability`** | `libs/rust/observability` | `0.1.0` | `nvbes-core`, `tracing-subscriber`, `opentelemetry`, `opentelemetry_sdk`, `opentelemetry-otlp`, `pyroscope`, `tracing-opentelemetry` |
| **`nvbes-platform`** | `libs/rust/platform` | `10.3.0` | - |
| **`nvbes-region`** | `libs/rust/region` | `0.1.0` | - |
| **`nvbes-test-utils`** | `libs/rust/test-utils` | `0.1.0` | `nvbes-trust-risk` |
| **`nvbes-trust-risk`** | `libs/rust/trust-risk` | `0.23.41` | `tonic` |

## 🟦 Bibliothèques TypeScript / SDKs (`libs/ts/`)

Total : **3 packages partagés**

| Package | Chemin | Version | Description / Rôle |
| :--- | :--- | :--- | :--- |
| **`@nvbes/account-sdk-core`** | `libs/ts/account-sdk-core` | `0.1.0` | OpenAPI specification for nvbes Account - cross-language SDK generation |
| **`@nvbes/email-ui`** | `libs/ts/email-ui` | `0.1.0` | React Email design system and transactional email templates for nvbes |
| **`@nvbes/identity-sdk-core`** | `libs/ts/identity-sdk-core` | `0.1.0` | OpenAPI specification for nvbes Identity - cross-language SDK generation |
