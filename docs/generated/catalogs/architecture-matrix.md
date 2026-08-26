---
title: Matrice d'Architecture & Dépendances
description: Graphe relationnel et frontières entre les services, workers et bibliothèques.
---

## Graphe de Dépendances Rust (Crates & Services)

Le diagramme ci-dessous illustre les liaisons déterministes entre les services applicatifs et les primitives partagées.

```mermaid
flowchart TD
    subgraph Applications ["Services & Workers"]
        nvbes_email_worker["nvbes-email-worker"]
        nvbes_trust_risk_service["nvbes-trust-risk-service"]
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
        nvbes_trust_risk["nvbes-trust-risk"]
    end

    nvbes_email_worker --> nvbes_core
    nvbes_email_worker --> nvbes_email
    nvbes_email_worker --> nvbes_email_scaleway
    nvbes_email_worker --> nvbes_observability
    nvbes_trust_risk_service --> nvbes_core
    nvbes_trust_risk_service --> nvbes_observability
    nvbes_trust_risk_service --> nvbes_trust_risk
    nvbes_trust_risk_service --> opentelemetry_otlp
```

## Règles d'Isolation des Domaines (Boundaries)

- **Isolation stricte** : Aucun service d'un domaine (ex: `identity`) ne peut importer directement la base de données d'un autre domaine (ex: `cloud`).
- **Communication inter-services** : Réalisée exclusivement via des contrats HTTP / OpenAPI typés ou des queues asynchrones.
- **Libs Core** : `libs/rust/core` et `libs/ts/web-runtime` doivent rester neutres et sans logique métier spécifique.
