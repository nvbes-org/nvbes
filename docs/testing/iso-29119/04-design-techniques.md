# ISO/IEC/IEEE 29119-4 — Techniques de conception de tests

> Catalogue des techniques ISO 29119-4 (boîte noire et boîte blanche)
> appliquées au codebase nvbes.

## 1. Vue d'ensemble

```
ISO 29119-4 Techniques
├── Boîte noire (Black-box)
│   ├── Équivalence par partition
│   ├── Valeurs limites
│   ├── Transition d'état
│   ├── Cause-effet
│   ├── Table de décision
│   ├── Cas d'utilisation
│   ├── Experience/Guessing
│   └── Risk-based
└── Boîte blanche (White-box)
    ├── Couverture de statement
    ├── Couverture de décision/branch
    ├── Couverture de condition
    ├── Couverture de path
    ├── Couverture de base path
    └── Mutation testing
```

## 2. Techniques boîte noire (Black-box)

### 2.1 Équivalence par partition (Equivalence Class Partitioning)

**Principe** : Diviser les inputs en classes où chaque valeur produit
le même comportement.

**nvbes implémentation** :

| Domaine | Input                      | Partitions                               | Fichiers tests                   |
| ------- | -------------------------- | ---------------------------------------- | -------------------------------- |
| Config  | `NVBES_JWT_EXPIRY_SECONDS` | valide (<0, 0, >0, max)                  | `identity.config.tests.rs`       |
| Config  | `NVBES_MAX_UPLOAD_SIZE_MB` | valide (0, 1, max, overflow)             | `email.worker.config.tests.rs`   |
| API     | Email address              | valide, vide, trop long, format invalide | `identity.auth.tests.rs`         |
| API     | Role                       | admin, member, viewer, invalide          | `identity.auth.tests.rs`         |
| Billing | Webhook signature          | valide, invalide, manquant               | `billing.webhooks.tests.rs`      |
| Trust   | Risk score                 | 0-100 valide, négatif, >100              | `trust_risk.assessment.tests.rs` |
| DB      | UUID                       | valide, vide, format invalide            | `account.database.tests.rs`      |

**Exemple Rust** :

```rust
// Partition équivalence pour la validation d'email
#[test]
fn test_email_validation_partitions() {
    // Partitions valides
    assert!(validate_email("user@example.com").is_ok());
    assert!(validate_email("a+b@c.fr").is_ok());

    // Partitions invalides
    assert!(validate_email("").is_err());           // vide
    assert!(validate_email("not-an-email").is_err()); // format
    assert!(validate_email(&"a".repeat(256)).is_err()); // trop long
}
```

### 2.2 Analyse des valeurs limites (Boundary Value Analysis)

**Principe** : Tester aux frontières des classes d'équivalence.

**nvbes implémentation** :

| Domaine    | Frontière                 | Tests                        | Fichiers                           |
| ---------- | ------------------------- | ---------------------------- | ---------------------------------- |
| Quota      | 0, max, max+1             | Upload bloque au dépassement | `account.database.tests.rs`        |
| TTL        | 0, 1, max, max+1          | Expiration lien partage      | `identity.tokens.tests.rs`         |
| Session    | 0, 1, max concurrent      | Invalidation sessions        | `identity.auth.tests.rs`           |
| Upload     | 0 bytes, 1 byte, max size | Validation taille            | `email.worker.config.tests.rs`     |
| Rate limit | 0, 1, limit, limit+1      | Throttling API               | `trust_risk.config.tests.rs`       |
| Retry      | 0, 1, max_retries         | Backoff worker               | `email.worker.dispatcher.tests.rs` |

**Exemple Rust** :

```rust
// Boundary value analysis pour quotas
#[test]
fn test_quota_boundary() {
    // Boundary: exactement à la limite
    assert_eq!(check_quota(999_999, 1_000_000), QuotaStatus::Available);
    assert_eq!(check_quota(1_000_000, 1_000_000), QuotaStatus::AtLimit);
    assert_eq!(check_quota(1_000_001, 1_000_000), QuotaStatus::Exceeded);
}
```

### 2.3 Transition d'état (State Transition Testing)

**Principe** : Modéliser le système comme un automate à états et tester
les transitions valides et invalides.

**nvbes implémentation** :

```
Auth Flow State Machine:
  ┌─────────┐   login    ┌──────────┐
  │ LoggedOut├──────────→│ AuthFactor│
  └─────────┘           │ Required  │
                        └─────┬─────┘
                              │ MFA/PASSKEY
                    ┌─────────┴─────────┐
                    ↓                   ↓
              ┌──────────┐       ┌──────────┐
              │ MFAVerify│       │Passkey   │
              │ (TOTP)   │       │Verify    │
              └─────┬────┘       └────┬─────┘
                    │                  │
                    └────────┬─────────┘
                             ↓
                      ┌──────────┐
                      │ Authenticated│
                      └─────┬────────┘
                            │
                    ┌───────┴────────┐
                    ↓                ↓
             ┌──────────┐    ┌──────────┐
             │ Active   │    │ StepUp   │
             │ Session  │    │ Required │
             └─────┬────┘    └─────┬────┘
                   │               │
                   └───────┬───────┘
                           ↓
                    ┌──────────┐
                    │ Revoked  │
                    └──────────┘
```

| Transition                         | Valide | Test                | Fichier                        |
| ---------------------------------- | ------ | ------------------- | ------------------------------ |
| LoggedOut → AuthFactorRequired     | Oui    | Login success       | `identity.auth.tests.rs`       |
| AuthFactorRequired → Authenticated | Oui    | MFA/Passkey success | `identity.mfa.crypto.tests.rs` |
| AuthFactorRequired → LoggedOut     | Oui    | MFA timeout/failure | `identity.auth.tests.rs`       |
| Active → Revoked                   | Oui    | Session revoke      | `identity.auth.tests.rs`       |
| Revoked → Active                   | Non    | Impossible          | —                              |
| Active → StepUpRequired            | Oui    | Sensible action     | `identity.tokens.tests.rs`     |
| StepUpRequired → Active            | Oui    | Step-up success     | `identity.tokens.tests.rs`     |

### 2.4 Cause-effet (Cause-Effect Graphing)

**Principe** : Modéliser les combinaisons de causes (inputs) et effets (outputs).

**nvbes implémentation** :

```
Webhook Billing:
  Causes:                          Effets:
  C1: Signature valide             E1: Webhook accepté (200)
  C2: Payload JSON valide          E2: Webhook rejeté (400)
  C3: Idempotence key présente     E3: Doublon ignoré (200)
  C4: Event type reconnu           E4: Événement traité
  C5: Timestamp dans fenêtre       E5: Webhook expiré (410)

  C1 ∧ C2 ∧ C3 ∧ C4 ∧ C5 → E1 ∧ E4
  ¬C1 → E2
  C1 ∧ C2 ∧ ¬C3 → E3
  C1 ∧ C2 ∧ C3 ∧ ¬C5 → E5
```

| Combinaison            | Causes          | Effet attendu    | Fichier                     |
| ---------------------- | --------------- | ---------------- | --------------------------- |
| Webhook valide complet | C1+C2+C3+C4+C5  | 200 + traitement | `billing.webhooks.tests.rs` |
| Signature invalide     | ¬C1             | 400              | `billing.webhooks.tests.rs` |
| JSON invalide          | C1+¬C2          | 400              | `billing.webhooks.tests.rs` |
| Doublon                | C1+C2+¬C3       | 200 (idempotent) | `billing.webhooks.tests.rs` |
| Event inconnu          | C1+C2+C3+¬C4    | 200 (ignoré)     | `billing.webhooks.tests.rs` |
| Timestamp expiré       | C1+C2+C3+C4+¬C5 | 410              | `billing.webhooks.tests.rs` |

### 2.5 Table de décision (Decision Table Testing)

**Principe** : Compiler toutes les combinaisons de conditions en une table.

**nvbes implémentation** :

```
| Condition                | R1 | R2 | R3 | R4 |
|--------------------------|----|----|----|----|
| Token valide             | Y  | Y  | N  | Y  |
| Role suffisant           | Y  | N  | -  | Y  |
| Step-up requis           | -  | -  | -  | Y  |
| Step-up validé           | -  | -  | -  | Y  |
|--------------------------|----|----|----|----|
| Action: Accès accordé    | X  |    |    | X  |
| Action: 403 Forbidden    |    | X  |    |    |
| Action: 401 Unauthorized |    |    | X  |    |
| Action: 428 Step-Up      |    |    |    |    |
```

### 2.6 Cas d'utilisation (Use Case Testing)

**Principe** : Tester les scénarios complets d'utilisation.

**nvbes implémentation** :

| Use case    | Scénarios                                   | Niveau      | Fichiers                              |
| ----------- | ------------------------------------------- | ----------- | ------------------------------------- |
| Inscription | Happy path, email invalide, doublon         | E2E         | `identity.auth.tests.rs` + Playwright |
| Upload      | Happy path, quota dépassé, fichier invalide | E2E         | `account.database.tests.rs` + E2E     |
| Partage     | Créer, accès public, expirer, révoquer      | Integration | `identity.tokens.tests.rs`            |
| Facturation | Checkout, upgrade, webhook, échec           | Integration | `billing.webhooks.tests.rs`           |
| MFA setup   | Ajouter TOTP, ajouter passkey, recovery     | E2E         | `identity.mfa.crypto.tests.rs`        |

### 2.7 Error Guessing (Experience-based)

**Principe** : Utiliser l'expérience pour cibler les zones à bugs probables.

**nvbes cibles prioritaires** :

| Zone à risque        | Raison                 | Tests                | Fichiers                       |
| -------------------- | ---------------------- | -------------------- | ------------------------------ |
| Concurrent access    | Race conditions DB     | Concurrency tests    | `account.database.tests.rs`    |
| Serialization JSON   | Edge cases, Unicode    | Fuzz                 | `fuzz/fuzz_targets/`           |
| Token expiry         | Clock skew, rotation   | Boundary             | `identity.tokens.tests.rs`     |
| Webhook replay       | Idempotence            | Duplicate processing | `billing.webhooks.tests.rs`    |
| Upload integrity     | Checksum, partial      | corruption scenarios | `email.worker.config.tests.rs` |
| Session invalidation | Password change, theft | Cascading revoke     | `identity.auth.tests.rs`       |

### 2.8 Risk-based Testing

**Principe** : Prioriser les tests selon le risque business/technique.

**nvbes matrice de risque** :

| Domaine                      | Impact   | Probabilité | Risque     | Priorité test |
| ---------------------------- | -------- | ----------- | ---------- | ------------- |
| Auth/MFA                     | Critique | Moyenne     | **Élevé**  | P0            |
| Billing webhooks             | Critique | Faible      | **Moyen**  | P0            |
| Data privacy (export/delete) | Critique | Faible      | **Moyen**  | P0            |
| Upload/quota                 | Élevé    | Moyenne     | **Élevé**  | P0            |
| Email dispatch               | Élevé    | Moyenne     | **Élevé**  | P0            |
| Public sharing               | Élevé    | Moyenne     | **Élevé**  | P0            |
| Session management           | Élevé    | Moyenne     | **Élevé**  | P0            |
| Trust/risk scoring           | Moyen    | Faible      | **Faible** | P1            |
| API rate limiting            | Moyen    | Moyenne     | **Moyen**  | P1            |
| Audit logging                | Moyen    | Faible      | **Faible** | P2            |
| UI states                    | Moyen    | Moyenne     | **Moyen**  | P1            |
| Container contracts          | Faible   | Faible      | **Faible** | P2            |

## 3. Techniques boîte blanche (White-box)

### 3.1 Couverture de statement (Statement Coverage)

**Principe** : Chaque ligne de code doit être exécutée au moins une fois.

**nvbes implémentation** :

La couverture de statement est mesurée sur tout le workspace Cargo avec des
seuils par crate consignés dans `docs/testing/rust-coverage-thresholds.json`
(seuil = baseline mesuré le 2026-09-07, commit `71b33339`, moins 1 point).
Crates exclus : `nvbes-scan`, `nvbes-product-cloud`, `nvbes-product-developer`,
`nvbes-product-enterprise` (crates squelettes sans logique testable) et
`nvbes-test-utils` (support de tests uniquement).

| Crate                               | Seuil lignes                         | Outil          | Commande                  |
| ----------------------------------- | ------------------------------------ | -------------- | ------------------------- |
| Tous les crates actifs du workspace | voir `rust-coverage-thresholds.json` | cargo llvm-cov | `pnpm test:rust:coverage` |

```bash
# Mesurer la couverture de statement du workspace et valider les seuils par crate
pnpm test:rust:coverage
# Rapport JSON brut généré
# .temp/rust/coverage-workspace.json
```

### 3.2 Couverture de décision/branch (Branch Coverage)

**Principe** : Chaque branche (if/else, match) doit être prise des deux côtés.

**nvbes implémentation** :

La couverture de décision est mesurée dans la même passe `llvm-cov` que la
couverture de statement ; les seuils functions/regions par crate sont également
consignés dans `docs/testing/rust-coverage-thresholds.json`.

```bash
# Couverture de statement, de fonction et de région en une passe, avec validation des seuils
pnpm test:rust:coverage
```

### 3.3 Couverture de condition

**Principe** : Chaque sous-condition booléenne individuelle doit être vraie
et fausse.

**nvbes implémentation** :

La métrique `branches` de `cargo llvm-cov --branch` mesure les branches
effectivement instrumentées, et non une preuve générale de couverture de
chaque condition. Elle ne démontre pas MC/DC (effet indépendant de chaque
condition sur la décision). L'étalonnage booléen reste à exécuter ; le rapport
`.temp/rust/coverage-condition-workspace.json` porte par fichier
`summary.branches { count, covered }`. Le score d'une crate = branches couvertes
/ branches instrumentées × 100.

> **Exigence toolchain** : `--branch` (alias `-Z coverage-options=branch`)
> n'émet la métrique branches **que sous un toolchain nightly**.
> `RUSTC_BOOTSTRAP=1` sur stable compile le flag mais produit 0 branche (aucune
> branch region émise), et une release stable rejette `-Z`. Le gate applique
> donc une vérification du toolchain épinglé `nightly-2026-09-09`
> et bascule l'exécution dessus
> via `rustup run`. Out-of-band : le gate n'est pas exécuté en CI (lane rust
> pinnée sur stable 1.98.1), il est lancé explicitement en local.

Le script historique lit `docs/testing/rust-condition-thresholds.json`.
Les seuils ci-dessous sont des baselines de diagnostic, **pas les critères
V1** : la release impose `max(90 %, seuil supérieur existant)` sur chaque
dépendance de production. Exclure une crate parce que son score est faible
est interdit. La migration du collecteur historique reste ouverte dans
[le dossier V1](../v1/README.md).

| Crate                   | Seuil branches | Outil                             | Commande                   |
| ----------------------- | -------------- | --------------------------------- | -------------------------- |
| nvbes-email             | 86 %           | cargo llvm-cov --branch (nightly) | `pnpm test:rust:condition` |
| nvbes-email-scaleway    | 95 %           | cargo llvm-cov --branch (nightly) | `pnpm test:rust:condition` |
| nvbes-identity-sdk      | 63 %           | cargo llvm-cov --branch (nightly) | `pnpm test:rust:condition` |
| nvbes-product-account   | 61 %           | cargo llvm-cov --branch (nightly) | `pnpm test:rust:condition` |
| nvbes-dpop              | 60 %           | cargo llvm-cov --branch (nightly) | `pnpm test:rust:condition` |
| nvbes-trust-risk        | 53 %           | cargo llvm-cov --branch (nightly) | `pnpm test:rust:condition` |
| nvbes-product-analytics | 46 %           | cargo llvm-cov --branch (nightly) | `pnpm test:rust:condition` |

```bash
# Mesurer la couverture de condition du workspace (nightly) et valider les seuils par crate
pnpm test:rust:condition
# Equivalent Nx : pnpm exec nx run rust-workspace:condition
# Rapport JSON brut généré
# .temp/rust/coverage-condition-workspace.json
```

```rust
// Test de couverture de condition
fn process_upload(
    size: u64,
    quota_remaining: u64,
    checksum_valid: bool,
) -> Result<(), UploadError> {
    // Condition composée: size <= quota_remaining && checksum_valid
    // → 4 sous-conditions instrumentées : size<=quota=V/F, checksum=V/F
    if size <= quota_remaining && checksum_valid {
        // Branche 1: les deux vrais
        Ok(())
    } else if size > quota_remaining {
        // Branche 2: quota dépassé
        Err(UploadError::QuotaExceeded)
    } else {
        // Branche 3: checksum invalide
        Err(UploadError::InvalidChecksum)
    }
}

// Tests nécessaires pour couvrir toutes les conditions:
// size <= quota_remaining = V, checksum_valid = V → Ok
// size <= quota_remaining = V, checksum_valid = F → InvalidChecksum
// size <= quota_remaining = F, checksum_valid = V → QuotaExceeded
// size <= quota_remaining = F, checksum_valid = F → QuotaExceeded (prioritaire)
```

### 3.4 Couverture de path

**Principe** : Tous les chemins d'exécution possibles doivent être testés.

**nvbes implémentation** :

Pour les fonctions avec chemins multiples (auth flow, upload flow, billing
reconciliation), la couverture de path est validée par :

1. Tests unitaires couvrant chaque combinaison de branches
2. Fuzz testing explorant les chemins non couverts
3. `cargo llvm-cov` mesurant la couverture globale

### 3.5 Couverture de base path (Basis Path)

**Principe** : Tester un ensemble indépendant de chemins couvrant toutes
les combinaisons de branches.

**nvbes implémentation** :

```rust
// Base path analysis pour auth flow
// Nombre cyclomatique M = nombre de décisions + 1
//
// fn authenticate(email, password, mfa_token) -> Result<Session, AuthError>
//   M = 5 (email valid, password valid, MFA required, MFA valid, session created)
//   → 5 chemins indépendants minimum
//
// Path 1: email invalide → AuthError::InvalidCredentials
// Path 2: email valide, password invalide → AuthError::InvalidCredentials
// Path 3: email+password valide, MFA requis, MFA invalide → AuthError::MFAFailed
// Path 4: email+password valide, MFA requis, MFA valide → Ok(Session)
// Path 5: email+password valide, MFA non requis → Ok(Session)
```

### 3.6 Mutation Testing

**Principe** : Introduire des mutations (bugs) dans le code et vérifier que
les tests les détectent.

**nvbes implémentation** :

| Outil         | Usage                                      | Scope                 |
| ------------- | ------------------------------------------ | --------------------- |
| cargo-mutants | Mutation testing Rust                      | Libraries core (gate) |
| manuel        | Modifier des asserts, vérifier sensibilité | Revue de tests        |

Le mutation score est `caught / (caught + missed + timeout)` : un mutant qui
pend (timeout) compte comme non détecté, les mutants non viables sont exclus.
Le gate est piloté par `docs/testing/rust-mutation-thresholds.json` : chaque
crate de périmètre a un seuil (baseline mesurée − 5 points), les crates sous
le plancher de 50 % sont exclues avec raison explicite et baseline conservée
pour suivre la régression.

```bash
# Installer cargo-mutants (une fois)
cargo install cargo-mutants --locked --version 27.1.0

# Lancer le gate de mutation sur les crates core puis valider les seuils
pnpm test:rust:mutation
# Execution locale parallele optionnelle : NVBES_MUTATION_JOBS=4 pnpm test:rust:mutation
# Equivalent Nx : pnpm exec nx run rust-workspace:mutation
# Résultats complets : .temp/rust/mutants.out/ (outcomes.json, log/, diff/)

# Cible ponctuelle sur une crate du périmètre
cargo mutants --package nvbes-email --timeout 600
```

## 4. Matrice de couverture par technique

| Technique         | Rust                          | TypeScript  | E2E         | k6      | Fuzz         |
| ----------------- | ----------------------------- | ----------- | ----------- | ------- | ------------ |
| Équivalence       | `#[test]`                     | `node:test` | Playwright  | —       | —            |
| Valeurs limites   | `#[test]`                     | `node:test` | Playwright  | —       | —            |
| Transition état   | `#[test]`                     | —           | Playwright  | —       | —            |
| Cause-effet       | `#[test]`                     | `node:test` | —           | —       | —            |
| Table décision    | `#[test]`                     | `node:test` | —           | —       | —            |
| Cas d'utilisation | Integration                   | —           | Playwright  | —       | —            |
| Error guessing    | `#[test]`                     | —           | —           | —       | `cargo-fuzz` |
| Risk-based        | Prioritaire                   | Prioritaire | Prioritaire | Profils | 3 cibles     |
| Statement cov.    | `llvm-cov`                    | —           | —           | —       | —            |
| Branch cov.       | `llvm-cov`                    | —           | —           | —       | —            |
| Condition cov.    | `llvm-cov --branch` (nightly) | —           | —           | —       | —            |
| Path cov.         | `llvm-cov` + fuzz             | —           | —           | —       | —            |
| Mutation          | cargo-mutants                 | —           | —           | —       | —            |

## 5. Sélection de techniques par type de test

### Tests unitaires Rust

| Technique prioritaire     | Justification                          |
| ------------------------- | -------------------------------------- |
| Équivalence par partition | Validation inputs (config, API params) |
| Valeurs limites           | Quotas, TTL, size limits               |
| Branch coverage           | `cargo llvm-cov` 90-95%                |
| Error guessing            | Fuzz targets existants                 |

### Tests d'intégration

| Technique prioritaire | Justification                           |
| --------------------- | --------------------------------------- |
| Cause-effet           | Webhook sign + idempotence + event type |
| Transition d'état     | Auth flow complet, session lifecycle    |
| Table de décision     | RBAC permissions matrix                 |
| Risk-based            | Priorisation via regression-matrix      |

### Tests E2E (Playwright)

| Technique prioritaire | Justification                  |
| --------------------- | ------------------------------ |
| Cas d'utilisation     | Parcours critiques utilisateur |
| Transition d'état     | Login/MFA/OAuth flows          |
| Error guessing        | Offline, latence, 429, 5xx     |

### Tests de performance (k6)

| Technique prioritaire | Justification                  |
| --------------------- | ------------------------------ |
| Risk-based            | Profils adaptés au risque      |
| Valeurs limites       | Charge nominale, spike, stress |
| Boundary              | SLO p95/p99 thresholds         |

### Tests de sécurité

| Technique prioritaire | Justification                 |
| --------------------- | ----------------------------- |
| Error guessing        | OWASP Top 10 patterns         |
| Fuzz                  | Input malformé, injection     |
| Risk-based            | Priorisation assets critiques |
