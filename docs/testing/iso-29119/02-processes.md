# ISO/IEC/IEEE 29119-2 — Processus de test

> Processus de test ISO 29119-2 mappés sur l'infrastructure CI/CD nvbes.

## 1. Vue d'ensemble des processus

```
┌─────────────────────────────────────────────────────────────────┐
│                    Processus de Management                       │
│  (monitoring, reporting, continu improvement)                    │
├──────────┬──────────┬──────────┬──────────┬──────────┬──────────┤
│ Planning │ Analysis │ Design   │ Implement│ Execute  │Complete  │
│          │          │          │          │          │          │
│ strategy │ risk     │ test     │ write    │ CI lanes │ gates    │
│ test plan│ analysis │ cases    │ code     │ manual   │ reports  │
│ resource │ coverage │ env      │ fixtures │ sessions │ archive  │
└──────────┴──────────┴──────────┴──────────┴──────────┴──────────┘
```

## 2. Processus fondamentaux

### 2.1 Test Planning

**ISO 29119** : Définir les objectifs, scope, approche, ressources et calendrier.

**nvbes implémentation** :

| Activité                    | Artefact                                                                             | Commande                               |
| --------------------------- | ------------------------------------------------------------------------------------ | -------------------------------------- |
| Stratégie globale V1 (MTP)  | `docs/testing/test-strategy.md`                                                      | —                                      |
| Level Test Plan par domaine | `docs/testing/{account,identity,billing,email,trust-risk,platform}-test-strategy.md` | —                                      |
| Manifeste machine-readable  | `docs/testing/account-test-manifest.json`                                            | `pnpm check:account-release-readiness` |
| Matrice de régression       | `docs/testing/regression-matrix.md`                                                  | —                                      |
| UI strategy                 | `docs/testing/ui-testing.md`                                                         | —                                      |
| UX protocol                 | `docs/testing/ux-testing.md`                                                         | —                                      |

**Entrée** : Direction produit V1, architecture système, risk analysis.
**Sortie** : Stratégies documentées, manifeste validé, matrice de régression.

### 2.2 Test Monitoring and Control

**ISO 29119** : Suivi des progrès, reporting, actions correctives.

**nvbes implémentation** :

| Monitoring               | Mécanisme                                     | Fréquence                |
| ------------------------ | --------------------------------------------- | ------------------------ |
| CI pipeline status       | GitHub Actions status checks                  | Chaque PR                |
| Coverage trends          | `cargo llvm-cov` workspace + seuils par crate | Chaque PR (lane rust CI) |
| Nx task cache            | `nx affected`                                 | Chaque PR                |
| k6 performance baselines | `tools/load-tests/account/`                   | Scheduled daily/weekly   |
| Security scans           | OSV, cargo-deny, gitleaks                     | Scheduled + CI           |
| Release readiness        | `pnpm check:account-release-readiness`        | Pre-release              |

**Sortie** : Rapports CI, coverage deltas, performance trends, security alerts.

### 2.3 Test Analysis

**ISO 29119** : Analyser les exigences, identifier les risques, définir les
critères d'entrée/sortie.

**nvbes implémentation** :

| Activité             | Outil/Méthode                                                              |
| -------------------- | -------------------------------------------------------------------------- |
| Risk-based analysis  | `regression-matrix.md` — priorisation par domaine critique                 |
| Requirement coverage | `account-test-manifest.json` — mapping catégorie → lane                    |
| Gap analysis         | Level Test Plans par domaine, § Limites explicites                         |
| Coverage gap         | `rust-coverage-thresholds.json` — seuils par crate (baseline + validation) |
| Security analysis    | OWASP Top 10 mapping (`pnpm check:security`)                               |

**Critères d'entrée par lane** :

| Lane        | Entry criteria                                         |
| ----------- | ------------------------------------------------------ |
| PR CI       | Format vert, lint vert, types valides                  |
| Unit tests  | `cargo test --workspace --lib --bins` vert             |
| Integration | DB migration valide, provider mocks prêts              |
| E2E         | Services déployés, URLs `NVBES_*_BASE_URL` configurées |
| k6          | Origin allowlistée, dataset synthétique prêt           |
| Staging     | Toutes les lanes PR vertes                             |
| Production  | Staging vert, approval signé, smoke vert               |

### 2.4 Test Design

**ISO 29119** : Concevoir les cas de test, sélectionner les techniques,
prioriser.

**nvbes implémentation** : Détail dans [04-design-techniques.md](04-design-techniques.md).

| Technique                | Cible nvbes                             |
| ------------------------ | --------------------------------------- |
| Equivalence partitioning | Validation inputs (config, API params)  |
| Boundary value analysis  | Quotas, limits, TTL, expiry             |
| State transition         | Auth flows (MFA, sessions, OAuth)       |
| Decision coverage        | Branches Rust (`cargo llvm-cov`)        |
| Risk-based               | Priorisation via `regression-matrix.md` |
| Use case                 | E2E parcours critiques                  |
| Error guessing           | Fuzz targets                            |

### 2.5 Test Implementation

**ISO 29119** : Écrire les tests, préparer les données et environnements.

**nvbes implémentation** :

| Activité          | Pattern                  | Fichiers                              |
| ----------------- | ------------------------ | ------------------------------------- |
| Unit tests inline | `#[cfg(test)] mod tests` | `src/*.tests.rs`                      |
| Integration tests | `tests/` directory       | `tests/container-contract.test.mjs`   |
| Fuzz targets      | `cargo-fuzz`             | `fuzz/fuzz_targets/*.rs`              |
| k6 scenarios      | JS load test scripts     | `tools/load-tests/account/*.js`       |
| E2E scripts       | Playwright specs         | `apps/identity-web/e2e/*.spec.ts`     |
| Test fixtures     | DB seeds, mocks          | `scripts/lib/test-env.sh`             |
| Test capture      | Email mock               | `libs/rust/email/src/test_capture.rs` |

**Patterns de test Rust** :

```rust
// Unit test inline (fichier: identity.auth.tests.rs)
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_token_validation_rejects_expired() {
        // Arrange
        let expired_token = create_expired_jwt();

        // Act
        let result = validate_token(&expired_token).await;

        // Assert
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TokenError::Expired));
    }
}
```

```rust
// Integration test (fichier: tests/container-contract.test.mjs)
import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

test("Dockerfile exposes correct port", () => {
    const dockerfile = readFileSync("Dockerfile", "utf8");
    assert.ok(dockerfile.includes("EXPOSE 3000"));
});
```

### 2.6 Test Execution

**ISO 29119** : Exécuter les tests, enregistrer les résultats, gérer les
défaillances.

**nvbes implémentation** :

| Lane          | Déclencheur             | Commande                                | Artefact          |
| ------------- | ----------------------- | --------------------------------------- | ----------------- |
| Unit          | PR                      | `cargo test --workspace --lib --bins`   | Cargo test output |
| Integration   | PR (DB required)        | `cargo test --workspace --tests`        | Test output       |
| Contract      | PR (contracts required) | `node tools/ci/run-lane.mjs contracts`  | Nx report         |
| TypeScript    | PR (TS required)        | `node tools/ci/run-lane.mjs typescript` | Nx report         |
| E2E           | Scheduled/Manual        | `pnpm test:e2e:critical`                | Playwright HTML   |
| Smoke         | Post-deploy             | `pnpm test:smoke`                       | HTTP assertions   |
| Load (daily)  | Cron `17 1 * * *`       | k6 `load` profile                       | k6 summary JSON   |
| Load (weekly) | Cron `31 1 * * 0`       | k6 `volume/spike/stress`                | k6 summary JSON   |
| DAST          | Cron `41 2 * * 3`       | ZAP scan                                | ZAP report        |
| Security      | Scheduled               | `pnpm check:security`                   | Security report   |
| Fuzz          | Manual                  | `pnpm security:fuzz`                    | Fuzz artifacts    |

**Gestion des défaillances** :

```bash
# Diagnostic unit test échoué
cargo test --workspace --lib --bins -- --nocapture 2>&1 | tail -50

# Diagnostic integration test
cargo test --workspace --tests -- --nocapture 2>&1 | tail -50

# Diagnostic E2E
pnpm test:e2e:critical -- --headed --debug

# Diagnostic k6
cat tools/load-tests/account/summary.json | jq '.metrics.http_req_failed'
```

### 2.7 Test Completion

**ISO 29119** : Finaliser les tests, produire les rapports, documenter les
résultats, archiver.

**nvbes implémentation** :

| Activité                | Artefact                         | Gate                                   |
| ----------------------- | -------------------------------- | -------------------------------------- |
| Release gate staging    | `pnpm release:gate:staging`      | Build + Stripe preflight + smoke + E2E |
| Release gate production | `pnpm release:gate:production`   | Approval signé + smoke + monitoring    |
| Acceptation signée      | `account-acceptance-charters.md` | Paquet de preuve signé Ed25519         |
| Coverage report         | `cargo llvm-cov` HTML            | Thresholds minimum atteints            |
| Security clearance      | OSV + cargo-deny + gitleaks      | 0 vulnérabilité critique               |
| Documentation update    | CHANGELOG, README                | Changelog format Keep a Changelog      |

## 3. Processus de management

### 3.1 Test Management Process

| Activité                    | Outils                            | Fréquence             |
| --------------------------- | --------------------------------- | --------------------- |
| Priorisation risk-based     | `regression-matrix.md`            | Avant chaque release  |
| Resource allocation         | CI runner config                  | Mensuelle             |
| Progress tracking           | Nx affected, CI dashboards        | Continu               |
| Defect management           | GitHub issues + BFB               | Chaque bug            |
| Test environment management | Docker, Terraform                 | Avant chaque campagne |
| Test data management        | Seeds, fixtures, synthetic        | Par campagne          |
| Release readiness           | `check:account-release-readiness` | Pre-release           |

### 3.2 Risques de test

| Risque              | Impact             | Mitigation                                         |
| ------------------- | ------------------ | -------------------------------------------------- |
| Skip infrastructure | Faux positif       | Aucun skip ne devient vert                         |
| Flaky test          | Perte de confiance | Pas de retry qui transforme échec en succès        |
| Coverage drop       | Régression cachée  | Thresholds versionnés, baisse = décision de risque |
| Secret leak         | Sécurité           | `gitleaks-history`, redaction logs                 |
| Faux positif DAST   | Bruit              | ZAP custom rules, exclusions ciblées               |
| E2E instabilité     | Blocage release    | Stables, rapides, diagnostiquables                 |

## 4. Processus de dynamique de test

### 4.1 Test Process Tailoring

Le processus ISO 29119 est adapté au contexte nvbes :

| ISO 29119 générique       | Adaptation nvbes                                      |
| ------------------------- | ----------------------------------------------------- |
| Formal test plan document | Stratégies markdown + manifeste JSON                  |
| Manual test execution     | CI/CD automatisé + sessions manuelles pour acceptance |
| Defect tracking system    | GitHub Issues + BFB templates                         |
| Test management tool      | Nx orchestration + manifeste machine-readable         |
| Test environment lab      | Docker + PostgreSQL éphémère + staging cloud          |

### 4.2 Pipeline de test intégré

```
PR ouvert
  ├─ scope (diff classification via nx-cache-manager.mjs)
  ├─ contracts (si TS/API modifié)
  ├─ typescript (si TS modifié)
  ├─ rust (si Rust modifié)
  │   ├─ cargo check --workspace
  │   ├─ cargo test --workspace --lib --bins
  │   └─ cargo test --workspace --tests (si DB disponible)
  ├─ database (si migrations modifiées)
  ├─ terraform (si infra modifiée)
  ├─ containers (si Dockerfile modifié)
  └─ ci-gate (toutes les lanes planifiées vertes)

Post-merge / Scheduled
  ├─ k6 daily (load profile)
  ├─ k6 weekly (volume, spike, stress)
  ├─ DAST weekly (ZAP)
  ├─ Security daily (OSV, cargo-deny, gitleaks)
  └─ Browser daily (Playwright cross-browser)

Release staging
  ├─ release:gate:staging
  │   ├─ Build complet
  │   ├─ Preflight Stripe
  │   ├─ Smoke staging
  │   └─ E2E critiques staging
  └─ Acceptation humaine signée

Release production
  ├─ release:gate:production
  │   ├─ Paquet d'acceptation signé
  │   ├─ Attestation control-plane
  │   ├─ Smoke production
  │   └─ Monitoring sans alerte critique
  └─ Plan de rollback connu
```
