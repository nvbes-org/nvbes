---
title: Catalogue des ADRs (Architecture Decision Records)
description: Journal des décisions d'architecture prises sur la plateforme nvbes.
---

Total : **11 décisions enregistrées**

| Fichier ADR | Titre | Statut |
| :--- | :--- | :--- |
| [`0001-billing-provider-stripe.md`](/adr/0001-billing-provider-stripe/) | **ADR 0001 - Provider Billing V1** | ✅ Accepté |
| [`0002-platform-clean-rebuild.md`](/adr/0002-platform-clean-rebuild/) | **ADR 0002 - Reconstruction Plateforme Big Bang Zero Dette** | ✅ Accepté |
| [`0003-internal-billing-platform.md`](/adr/0003-internal-billing-platform/) | **ADR 0003 - Plateforme Billing Interne Multi-Provider** | ✅ Accepté |
| [`0004-provider-neutral-psp-routing.md`](/adr/0004-provider-neutral-psp-routing/) | **ADR 0004 - Routing PSP Provider-Neutral et Fallback Regional** | ✅ Accepté |
| [`0005-separate-identity-from-account.md`](/adr/0005-separate-identity-from-account/) | **ADR 0005 - Separate Identity from Account** | ✅ Accepté |
| [`0006-centralize-trust-risk-distribute-enforcement.md`](/adr/0006-centralize-trust-risk-distribute-enforcement/) | **ADR 0006 - Centraliser Trust/Risk et distribuer l'enforcement** | ✅ Accepté |
| [`0007-ephemeral-staging-lifecycle.md`](/adr/0007-ephemeral-staging-lifecycle/) | **ADR 0007 - Operate Staging as an Ephemeral Environment** | ✅ Accepté |
| [`0008-adopt-multi-shell-super-app.md`](/adr/0008-adopt-multi-shell-super-app/) | **ADR 0008 - Adopt a Multi-Shell Architecture for the Cloud and Account Super App** | ⚠️ superseded |
| [`0011-workspace-end-to-end-encryption.md`](/adr/0011-workspace-end-to-end-encryption/) | **ADR 0011 - Adopter un chiffrement end-to-end optionnel par workspace** | ⚠️ proposed |
| [`0012-adopt-scaleway-serverless-sql-per-product.md`](/adr/0012-adopt-scaleway-serverless-sql-per-product/) | **ADR 0012 - Adopter Scaleway Serverless SQL par produit** | ✅ Accepté |
| [`2026-07-05-account-cloud-service-taxonomy.md`](/adr/2026-07-05-account-cloud-service-taxonomy/) | **ADR 2026-07-05 - Account Cloud Service Taxonomy** | ⚠️ superseded |

## Pourquoi des ADRs ?
Les ADRs formalisent les choix structurants (changement de base de données, intégration d'un PSP, refonte de l'authentification). Tout changement architectural majeur doit faire l'objet d'un nouvel ADR avant implémentation.
