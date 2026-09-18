# ISO/IEC/IEEE 29119 - Implémentation nvbes

> **Statut : implémentation V1 en cours, release bloquée.** Le dispositif cible le runtime actif
> (Identity, Account, Billing, Email, Trust/Risk, Platform Operations).
> Les produits archivés (Cloud, Drive, Enterprise, Backoffice) ne sont pas
> couverts.

## Portée

Adaptation interne de la série ISO/IEC/IEEE 29119 (parties 1 à 5)
adaptée à l'architecture Rust/Axum + TypeScript/React du monorepo nvbes.

## Parties

| Partie                          | Document                                           | Statut |
| ------------------------------- | -------------------------------------------------- | ------ |
| 1 — Concepts                    | [01-concepts.md](01-concepts.md)                   | Actif  |
| 2 — Processus                   | [02-processes.md](02-processes.md)                 | Actif  |
| 3 — Documentation               | [03-documentation.md](03-documentation.md)         | Actif  |
| 4 — Techniques de conception    | [04-design-techniques.md](04-design-techniques.md) | Actif  |
| 5 — Tests pilotés par mots-clés | [05-keyword-driven.md](05-keyword-driven.md)       | Actif  |

## Mapping avec l'infrastructure existante

```
ISO 29119 Part 2 (Processus)
  └─→ docs/testing/test-strategy.md (stratégie V1)
  └─→ .github/workflows/ci.yml (CI lanes)
  └─→ scripts/release-gate.sh (gates de release)

ISO 29119 Part 3 (Documentation)
  └─→ docs/testing/v1/manifest.json (manifeste machine-readable V1)
  └─→ docs/testing/test-strategy.md (Master Test Strategy)
  └─→ docs/testing/account-test-strategy.md (Level Test Plan Account)
  └─→ docs/testing/identity-test-strategy.md (Level Test Plan Identity)
  └─→ docs/testing/billing-test-strategy.md (Level Test Plan Billing)
  └─→ docs/testing/email-test-strategy.md (Level Test Plan Email)
  └─→ docs/testing/trust-risk-test-strategy.md (Level Test Plan Trust/Risk)
  └─→ docs/testing/platform-test-strategy.md (Level Test Plan Platform Operations)
  └─→ docs/testing/regression-matrix.md (matrice de régression)

ISO 29119 Part 4 (Techniques)
  └─→ fuzz/fuzz_targets/ (boîte blanche: fuzz testing)
  └─→ libs/rust/*/src/*.property.tests.rs (boîte blanche: property-based testing)
  └─→ libs/rust/*/src/*.tests.rs (boîte blanche: unit tests)
  └─→ docs/testing/rust-coverage-thresholds.json (boîte blanche: seuils coverage par crate)
  └─→ apps/*/tests/container-contract.test.mjs (boîte noire: contract tests)
  └─→ tools/load-tests/account/ (boîte noire: performance)

ISO 29119 Part 5 (Keyword-driven)
  └─→ docs/testing/iso-29119/05-keyword-driven.md (framework design)
```

## Commandes associées

```bash
# Exécution de la suite complète
cargo test --workspace              # Tests unitaires + intégration Rust
pnpm test:unit                      # Unitaires (web + Rust)
pnpm test:integration               # Intégration Rust + Terraform
pnpm test:rust:coverage             # Couverture llvm-cov workspace + seuils par crate
pnpm test:e2e:critical              # E2E Playwright critiques
pnpm test:smoke                     # Smoke HTTP
pnpm check:security                 # Sécurité complète
pnpm verify                         # Vérification pré-commit complète
```

## Conformité

Chaque partie documente les exigences ISO 29119 applicables, mappe les contrôles
existants du projet, et identifie les gaps explicites. Les gaps sont classés :

- **release-blocking** : bloque la promotion production
- **known-gap** : documenté, owner assigné, date de revue
- **future** : hors périmètre V1

## Références

- [ISO/IEC/IEEE 29119-1:2022](https://www.iso.org/standard/81291.html) — General concepts
- [ISO/IEC/IEEE 29119-2:2021](https://www.iso.org/standard/79428.html) — Test processes
- [ISO/IEC/IEEE 29119-3:2021](https://standards.ieee.org/ieee/360/7499/) — Test documentation
- [ISO/IEC/IEEE 29119-4:2021](https://www.iso.org/standard/79430.html) — Test techniques
- [ISO/IEC/IEEE 29119-5:2024](https://www.iso.org/standard/87233.html) — Keyword-driven testing
- [Direction produit V1](../../product/nvbes-product-strategy.md)
- [Stratégie de test V1](../test-strategy.md)

Les éditions ont été vérifiées dans les catalogues publics ISO/IEEE le
2026-09-10. Les textes complets n'ont pas fait l'objet d'une revue clause par
clause : aucune certification ni conformité exhaustive n'est revendiquée.
Les seuils de 90 % sont des exigences nvbes, pas des seuils imposés par ISO.
Le manifeste actif est `docs/testing/v1/manifest.json`, et la décision commune
du rapport et de la release est `pnpm exec nx run test-summary:release`.
Le catalogue Account historique ne peut pas servir de preuve de clôture V1.
L'état d'implémentation et les limites du gate sont détaillés dans
[le dossier V1](../v1/README.md).
