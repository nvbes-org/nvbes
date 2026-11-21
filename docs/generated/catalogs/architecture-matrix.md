---
title: Matrice d'Architecture & Dépendances
description: Graphe relationnel et frontières entre les services, workers et bibliothèques.
---

## Graphe de Dépendances Rust (Crates & Services)

Le diagramme ci-dessous illustre les liaisons déterministes entre les services applicatifs et les primitives partagées.

```mermaid
flowchart TD
    subgraph Applications ["Services & Workers"]
        nvbes_api["nvbes-api"]
        nvbes_worker["nvbes-worker"]
    end

    subgraph Libraries ["Bibliotheques Partagees"]
        nvbes_account_service["nvbes-account-service"]
        nvbes_email_scaleway["nvbes-email-scaleway"]
        nvbes_audit["nvbes-audit"]
        nvbes_billing["nvbes-billing"]
        nvbes_billing_service["nvbes-billing-service"]
        nvbes_consent["nvbes-consent"]
        nvbes_core["nvbes-core"]
        nvbes_dpop["nvbes-dpop"]
        nvbes_email["nvbes-email"]
        nvbes_identity_service["nvbes-identity-service"]
        nvbes_identity_sdk["nvbes-identity-sdk"]
        nvbes_observability["nvbes-observability"]
        nvbes_platform["nvbes-platform"]
        nvbes_region["nvbes-region"]
        nvbes_test_utils["nvbes-test-utils"]
        nvbes_trust_risk["nvbes-trust-risk"]
    end

    nvbes_api --> nvbes_account_service
    nvbes_api --> nvbes_billing_service
    nvbes_api --> nvbes_consent
    nvbes_api --> nvbes_core
    nvbes_api --> nvbes_identity_service
    nvbes_api --> nvbes_observability
    nvbes_api --> tower_http
    nvbes_worker --> nvbes_account_service
    nvbes_worker --> nvbes_billing_service
    nvbes_worker --> nvbes_core
    nvbes_worker --> nvbes_email
    nvbes_worker --> nvbes_email_scaleway
    nvbes_worker --> nvbes_observability
```

## Règles d'Isolation des Domaines (Boundaries)

- **Isolation stricte** : Aucun service d'un domaine (ex: `identity`) ne peut importer directement la base de données d'un autre domaine (ex: `cloud`).
- **Communication inter-services** : Réalisée exclusivement via des contrats HTTP / OpenAPI typés ou des queues asynchrones.
- **Lib Core** : `libs/rust/core` doit rester neutre et sans logique métier spécifique.
