---
title: Matrice d'Architecture & Dépendances
description: Graphe relationnel et frontières entre les services, workers et bibliothèques.
---

## Graphe de Dépendances Rust (Crates & Services)

Le diagramme ci-dessous illustre les liaisons déterministes entre les services applicatifs et les primitives partagées.

```mermaid
flowchart TD
    subgraph Applications ["Services & Workers"]
        nvbes_account_service["nvbes-account-service"]
        nvbes_account_worker["nvbes-account-worker"]
        nvbes_backoffice_service["nvbes-backoffice-service"]
        nvbes_billing_service["nvbes-billing-service"]
        nvbes_billing_worker["nvbes-billing-worker"]
        nvbes_cloud_service["nvbes-cloud-service"]
        nvbes_cloud_worker["nvbes-cloud-worker"]
        nvbes_developer_service["nvbes-developer-service"]
        nvbes_email_worker["nvbes-email-worker"]
        nvbes_enterprise_service["nvbes-enterprise-service"]
        nvbes_gateway_cloud["nvbes-gateway-cloud"]
        nvbes_identity_service["nvbes-identity-service"]
        nvbes_identity_worker["nvbes-identity-worker"]
    end

    subgraph Libraries ["Bibliotheques Partagees"]
        nvbes_analytics_posthog["nvbes-analytics-posthog"]
        nvbes_email_scaleway["nvbes-email-scaleway"]
        nvbes_audit["nvbes-audit"]
        nvbes_billing["nvbes-billing"]
        nvbes_core["nvbes-core"]
        nvbes_dpop["nvbes-dpop"]
        nvbes_email["nvbes-email"]
        nvbes_identity_sdk["nvbes-identity-sdk"]
        nvbes_observability["nvbes-observability"]
        nvbes_platform["nvbes-platform"]
        nvbes_ports["nvbes-ports"]
        nvbes_product_analytics["nvbes-product-analytics"]
        nvbes_product_account["nvbes-product-account"]
        nvbes_product_cloud["nvbes-product-cloud"]
        nvbes_product_developer["nvbes-product-developer"]
        nvbes_product_enterprise["nvbes-product-enterprise"]
        nvbes_product_identity["nvbes-product-identity"]
        nvbes_redis["nvbes-redis"]
        nvbes_region["nvbes-region"]
        nvbes_scan["nvbes-scan"]
        nvbes_storage["nvbes-storage"]
        nvbes_tenancy["nvbes-tenancy"]
    end

    nvbes_account_service --> nvbes_identity_sdk
    nvbes_account_service --> nvbes_product_account
    nvbes_account_service --> nvbes_storage
    nvbes_account_service --> sqlx
    nvbes_account_service --> tower_http
    nvbes_account_service --> tracing_subscriber
    nvbes_account_worker --> nvbes_account_service
    nvbes_account_worker --> nvbes_core
    nvbes_account_worker --> nvbes_product_account
    nvbes_account_worker --> nvbes_storage
    nvbes_account_worker --> sqlx
    nvbes_backoffice_service --> nvbes_billing
    nvbes_backoffice_service --> nvbes_core
    nvbes_backoffice_service --> nvbes_email
    nvbes_backoffice_service --> nvbes_observability
    nvbes_backoffice_service --> tower_http
    nvbes_backoffice_service --> tracing_subscriber
    nvbes_billing_service --> nvbes_billing
    nvbes_billing_service --> nvbes_analytics_posthog
    nvbes_billing_service --> nvbes_core
    nvbes_billing_service --> nvbes_observability
    nvbes_billing_service --> nvbes_product_analytics
    nvbes_billing_service --> nvbes_product_account
    nvbes_billing_service --> nvbes_redis
    nvbes_billing_service --> tower_http
    nvbes_billing_worker --> nvbes_billing
    nvbes_billing_worker --> nvbes_billing_service
    nvbes_billing_worker --> nvbes_core
    nvbes_billing_worker --> nvbes_email
    nvbes_billing_worker --> nvbes_observability
    nvbes_billing_worker --> nvbes_analytics_posthog
    nvbes_billing_worker --> nvbes_product_analytics
    nvbes_billing_worker --> nvbes_redis
    nvbes_cloud_service --> nvbes_core
    nvbes_cloud_service --> nvbes_audit
    nvbes_cloud_service --> nvbes_redis
    nvbes_cloud_service --> nvbes_scan
    nvbes_cloud_service --> nvbes_storage
    nvbes_cloud_service --> nvbes_tenancy
    nvbes_cloud_service --> nvbes_observability
    nvbes_cloud_service --> nvbes_analytics_posthog
    nvbes_cloud_service --> nvbes_product_analytics
    nvbes_cloud_service --> nvbes_product_account
    nvbes_cloud_service --> nvbes_product_cloud
    nvbes_cloud_service --> nvbes_identity_sdk
    nvbes_cloud_service --> nvbes_region
    nvbes_cloud_service --> tower_http
    nvbes_cloud_service --> tracing_subscriber
    nvbes_cloud_service --> utoipa
    nvbes_cloud_service --> utoipa_swagger_ui
    nvbes_cloud_worker --> nvbes_core
    nvbes_cloud_worker --> nvbes_redis
    nvbes_cloud_worker --> nvbes_observability
    nvbes_cloud_worker --> nvbes_product_cloud
    nvbes_cloud_worker --> nvbes_region
    nvbes_cloud_worker --> nvbes_storage
    nvbes_developer_service --> nvbes_core
    nvbes_developer_service --> nvbes_observability
    nvbes_developer_service --> nvbes_product_identity
    nvbes_developer_service --> tower_http
    nvbes_developer_service --> tracing_subscriber
    nvbes_email_worker --> nvbes_core
    nvbes_email_worker --> nvbes_email
    nvbes_email_worker --> nvbes_email_scaleway
    nvbes_email_worker --> nvbes_observability
    nvbes_enterprise_service --> nvbes_core
    nvbes_enterprise_service --> nvbes_observability
    nvbes_enterprise_service --> tracing_subscriber
    nvbes_gateway_cloud --> nvbes_core
    nvbes_gateway_cloud --> nvbes_observability
    nvbes_gateway_cloud --> tower_http
    nvbes_identity_service --> nvbes_core
    nvbes_identity_service --> nvbes_dpop
    nvbes_identity_service --> nvbes_redis
    nvbes_identity_service --> nvbes_audit
    nvbes_identity_service --> nvbes_email
    nvbes_identity_service --> nvbes_region
    nvbes_identity_service --> nvbes_tenancy
    nvbes_identity_service --> nvbes_observability
    nvbes_identity_service --> nvbes_analytics_posthog
    nvbes_identity_service --> nvbes_product_analytics
    nvbes_identity_service --> nvbes_product_account
    nvbes_identity_service --> nvbes_product_cloud
    nvbes_identity_service --> nvbes_product_identity
    nvbes_identity_service --> nvbes_storage
    nvbes_identity_service --> sqlx
    nvbes_identity_service --> tower_http
    nvbes_identity_service --> tracing_subscriber
    nvbes_identity_service --> utoipa
    nvbes_identity_service --> utoipa_swagger_ui
    nvbes_identity_service --> webauthn_rs
    nvbes_identity_worker --> nvbes_email
    nvbes_identity_worker --> nvbes_core
    nvbes_identity_worker --> nvbes_storage
    nvbes_identity_worker --> nvbes_redis
    nvbes_identity_worker --> nvbes_observability
    nvbes_identity_worker --> nvbes_product_identity
    nvbes_identity_worker --> nvbes_product_analytics
    nvbes_identity_worker --> nvbes_region
    nvbes_identity_worker --> tonic
    nvbes_identity_worker --> tracing_subscriber
```

## Règles d'Isolation des Domaines (Boundaries)

- **Isolation stricte** : Aucun service d'un domaine (ex: `identity`) ne peut importer directement la base de données d'un autre domaine (ex: `cloud`).
- **Communication inter-services** : Réalisée exclusivement via des contrats HTTP / OpenAPI typés ou des queues asynchrones.
- **Libs Core** : `libs/rust/core` et `libs/ts/web-runtime` doivent rester neutres et sans logique métier spécifique.
