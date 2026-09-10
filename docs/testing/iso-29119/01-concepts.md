# ISO/IEC/IEEE 29119-1 — Concepts de test logiciel

> Mapping des concepts ISO 29119-1 vers le vocabulaire et les artefacts nvbes.

## 1. Concepts fondamentaux de test

### 1.1 Test logiciel

**ISO 29119** : Processus d'évaluation d'un objet de test par rapport aux
exigences et/ou croyances quant à la qualité attendue.

**nvbes** : Le processus de validation couvre l'ensemble des niveaux de la
pyramide de test — unitaire, intégration, E2E, smoke, performance, sécurité —
déployés via `cargo test`, `node --test`, Playwright, k6 et OWASP ZAP.

### 1.2 Objet de test (test item)

**ISO 29119** : Élément du produit testé (component, system, architecture).

**nvbes** : Chaque crate Rust (`identity-service`, `email-worker`,
`billing-service`, `trust-risk-service`, `account-service`) et chaque package
TypeScript (`identity-sdk-web`, `web-ui`, `account-client`) constitue un objet
de test. Les objets sont déclarés dans `Cargo.toml` workspace et
`pnpm-workspace.yaml`.

### 1.3 Niveaux de test

| ISO 29119           | nvbes               | Implémentation                                         |
| ------------------- | ------------------- | ------------------------------------------------------ |
| Unit testing        | Tests unitaires     | `cargo test --lib --bins`, `#[cfg(test)]` inline       |
| Integration testing | Tests d'intégration | `cargo test --tests`, `node --test *.test.mjs`         |
| System testing      | Tests système       | `pnpm test:e2e:critical` (Playwright)                  |
| Acceptance testing  | Tests d'acceptation | Alpha/Beta/UAT manuels (charters signés)               |
| Regression testing  | Tests de régression | `pnpm test:pre-build`, matrice `regression-matrix.md`  |
| Smoke testing       | Smoke tests         | `pnpm test:smoke`                                      |
| Performance testing | Tests de charge     | k6 profiles (smoke, load, volume, spike, stress, soak) |
| Security testing    | Tests de sécurité   | `pnpm check:security`, `pnpm security:fuzz`            |

### 1.4 Types de test

| ISO 29119              | nvbes                      | Outils                                          |
| ---------------------- | -------------------------- | ----------------------------------------------- |
| Functional testing     | Tests fonctionnels         | Unit, integration, E2E                          |
| Non-functional testing | Tests non-fonctionnels     | Performance, sécurité, accessibilité            |
| Structural testing     | Tests structurels          | Fuzz (boîte blanche), coverage `cargo llvm-cov` |
| Change-related testing | Tests liés aux changements | Regression, affected `nx affected`              |
| Black-box testing      | Boîte noire                | Contract tests, E2E, smoke                      |
| White-box testing      | Boîte blanche              | Unit tests internals, fuzz, coverage            |
| Grey-box testing       | Boîte grise                | Integration tests avec DB réal                  |

### 1.5 Processus de test

**ISO 29119** : Les processus fondamentaux (planning, monitoring, analysis,
design, implementation, execution, completion) et de management.

**nvbes** :映射é dans [02-processes.md](02-processes.md). Le processus est
orchestré par les CI lanes GitHub Actions, Nx run-many, et le release gate.

### 1.6 Testware

**ISO 29119** : Artefacts produits ou utilisés pendant le processus de test.

**nvbes** :

- **Test plan** : `docs/testing/test-strategy.md` (MTP), les 6 Level Test Plans par domaine
- **Test cases** : `#[test]` dans `src/*.tests.rs`, `*.test.mjs`
- **Test data** : seeds synthétiques, comptes de test, fixtures DB
- **Test environment** : PostgreSQL éphémère, Docker containers, staging
- **Test reports** : artefacts CI (Nx/Cargo/Playwright/k6 rapports)
- **Test manifest** : `account-test-manifest.json` (machine-readable)

### 1.7 Critères d'arrêt (exit criteria)

**ISO 29119** : Conditions devant être satisfaites avant de terminer une activité.

**nvbes** :

- `pnpm verify` vert (format + lint + check + test)
- Tous les gates de release passent
- Aucune vulnérabilité critique ouverte (`cargo audit`, OSV)
- Coverage minimum atteint (email: 95% lignes, identity/worker: 90%)
- Acceptation humaine signée (Alpha/Beta/UAT)

### 1.8 Test oracle

**ISO 29119** : Source de règles déterminant le comportement attendu.

**nvbes** :

- **Résultat attendu** : assertions dans les tests (`assert_eq!`, `assert!`)
- **Oracles automatiques** : coverage thresholds, contract validation, k6 SLO
- **Oracles humains** : charters d'acceptation signés, sessions usability
- **Oracles d'infrastructure** : container-contract tests, Terraform validate

### 1.9 Métakxritères de test (test criteria)

| ISO 29119           | nvbes                                     | Seuil                  |
| ------------------- | ----------------------------------------- | ---------------------- |
| Entry criteria      | Gates PR (lint, format, types, unitaires) | 100% vert              |
| Exit criteria       | Gates release (staging/production)        | 0 gap release-blocking |
| Completion criteria | Acceptation signée + smoke                | Verts                  |

## 2. Hiérarchie des documents de test

```
Master Test Plan (MTP)
  └─→ docs/testing/test-strategy.md (stratégie V1 globale)
  └─→ docs/testing/iso-29119/02-processes.md (processus ISO)

Level Test Plan (LTP)
  └─→ docs/testing/account-test-strategy.md (Account)
  └─→ docs/testing/identity-test-strategy.md (Identity)
  └─→ docs/testing/billing-test-strategy.md (Billing)
  └─→ docs/testing/email-test-strategy.md (Email)
  └─→ docs/testing/trust-risk-test-strategy.md (Trust/Risk)
  └─→ docs/testing/platform-test-strategy.md (Platform Operations)
  └─→ docs/testing/ui-testing.md (UI)
  └─→ docs/testing/ux-testing.md (UX)

Test Suite
  └─→ docs/testing/account-test-manifest.json (machine-readable)
  └─→ docs/testing/regression-matrix.md (scénarios critiques)

Test Case
  └─→ src/*.tests.rs (Rust unit/integration)
  └─→ tests/*.test.mjs (Node contract)
  └─→ tools/load-tests/*.js (k6 performance)
  └─→ archive/apps/*/e2e/*.spec.ts (Playwright E2E)

Test Report
  └─→ CI artifacts (Nx/Cargo/Playwright/k6)
  └─→ docs/testing/account-acceptance-charters.md (signé)
```

## 3. Rôles et responsabilités

| ISO 29119          | nvbes                    | Responsable            |
| ------------------ | ------------------------ | ---------------------- |
| Test Manager       | Orchestrateur release    | QA/owner technique     |
| Test Analyst       | Conception des scénarios | Engineering + Product  |
| Test Designer      | Techniques de conception | Engineering            |
| Test Executor      | Exécution des tests      | CI/CD + opérateur      |
| Test Builder       | Construction testware    | Engineering            |
| Test Administrator | Environnement de test    | SRE/DevOps             |
| Test Reviewer      | Revue des artefacts      | Security/Privacy owner |

## 4. Classification des tests nvbes par ISO 29119

### Par objectif

| Objet de test              | Type ISO        | Niveau             | Automatisé |
| -------------------------- | --------------- | ------------------ | ---------- |
| identity-service auth      | Fonctionnel     | Unit + Integration | Oui        |
| email-worker dispatch      | Fonctionnel     | Unit + Integration | Oui        |
| billing-service webhooks   | Fonctionnel     | Unit + Integration | Oui        |
| trust-risk assessment      | Fonctionnel     | Unit + Integration | Oui        |
| account-service DB/RLS     | Fonctionnel     | Unit + Integration | Oui        |
| Container contracts        | Non-fonctionnel | System             | Oui        |
| API contracts (OpenAPI)    | Fonctionnel     | System             | Oui        |
| Load performance           | Non-fonctionnel | System             | Oui        |
| Security (OWASP)           | Non-fonctionnel | System             | Oui        |
| Browser E2E                | Fonctionnel     | System             | Oui        |
| Fuzz (DPoP, OAuth, Upload) | Structurel      | Unit               | Oui        |
| Accessibility              | Non-fonctionnel | System             | Semi       |
| Alpha/Beta/UAT             | Fonctionnel     | Acceptance         | Non        |
| Pentest                    | Non-fonctionnel | Acceptance         | Non        |

### Par technique de conception

| Technique ISO 29119-4    | nvbes                      | Fichiers                                                                        |
| ------------------------ | -------------------------- | ------------------------------------------------------------------------------- |
| Equivalence partitioning | Validation inputs          | `identity.config.tests.rs`, `trust_risk.rules.tests.rs`                         |
| Boundary value analysis  | Quotas, limits, expiry     | `billing.webhooks.tests.rs`, `email.worker.config.tests.rs`                     |
| Decision coverage        | Branches Rust              | `cargo llvm-cov` workspace (seuils par crate : `rust-coverage-thresholds.json`) |
| Statement coverage       | Lignes Rust                | `cargo llvm-cov` workspace (seuils par crate : `rust-coverage-thresholds.json`) |
| State transition         | Auth flow (MFA, sessions)  | `identity.auth.tests.rs`, `identity.mfa.crypto.tests.rs`                        |
| Cause-effect graphing    | Webhook sign + idempotence | `billing.webhooks.tests.rs`                                                     |
| Random/chaos testing     | Fuzz                       | `fuzz/fuzz_targets/`                                                            |
| Use case testing         | E2E critiques              | `apps/identity-web/e2e/*.spec.ts`                                               |
| Risk-based testing       | Priorisation matrice       | `regression-matrix.md`                                                          |
