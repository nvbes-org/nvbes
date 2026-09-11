# ISO/IEC/IEEE 29119-3 — Documentation de test

> Templates et structures de documentation ISO 29119-3 pour nvbes.

## 1. Hiérarchie des documents

```
Master Test Plan (MTP)
  ├─ Test Strategy (globale) → docs/testing/test-strategy.md
  ├─ Level Test Plans (par domaine)
  │   ├─ Test Plan Account → docs/testing/account-test-strategy.md
  │   ├─ Test Plan Identity → docs/testing/identity-test-strategy.md
  │   ├─ Test Plan Billing → docs/testing/billing-test-strategy.md
  │   ├─ Test Plan Email → docs/testing/email-test-strategy.md
  │   ├─ Test Plan Trust/Risk → docs/testing/trust-risk-test-strategy.md
  │   └─ Test Plan Platform Operations → docs/testing/platform-test-strategy.md
  ├─ Test Design Specifications
  │   ├─ Test Suite (catégorie de tests)
  │   │   ├─ Test Case (scénario individuel)
  │   │   └─ Test Data
  │   └─ Test Environment Specification
  ├─ Test Procedures (scripts d'exécution)
  └─ Test Reports
      ├─ Test Execution Report
      ├─ Test Summary Report
      └─ Acceptance Report (signé)
```

## 2. Master Test Plan (MTP)

### 2.1 Template

```markdown
# Master Test Plan — [Projet/Release]

## 1. Introduction

- 1.1 Objectif
- 1.2 Portée
- 1.3 Références

## 2. Approche de test

- 2.1 Niveaux de test
- 2.2 Types de test
- 2.3 Techniques de conception
- 2.4 Outils

## 3. Environnement de test

- 3.1 Environnements requis
- 3.2 Données de test
- 3.3 Infrastructure

## 4. Exécution

- 4.1 Déclencheurs
- 4.2 Critères d'entrée
- 4.3 Critères de sortie
- 4.4 Gestion des défaillances

## 5. Livrables

- 5.1 Rapports de test
- 5.2 Preuves d'acceptation
- 5.3 Documentation de régression

## 6. Planning

- 6.1 Jalons
- 6.2 Dépendances
- 6.3 Risques

## 7. Rôles et responsabilités

## 8. Approbation
```

### 2.2 Mapping nvbes

| Section       | Artefact nvbes                                                           |
| ------------- | ------------------------------------------------------------------------ |
| Objectif      | `docs/testing/test-strategy.md` § Objectif                               |
| Portée        | Runtime V1 : Identity, Account, Billing, Email, Trust/Risk, Platform Ops |
| Approche      | Pyramide de test V1 (§ Pyramide cible)                                   |
| Environnement | Docker, PostgreSQL, staging cloud                                        |
| Exécution     | CI lanes (`.github/workflows/ci.yml`)                                    |
| Livrables     | CI artifacts, acceptance charters signés                                 |
| Planning      | Release gates staging/production                                         |

## 3. Test Design Specification

### 3.1 Template — Test Suite

```markdown
# Test Suite: [Catégorie]

## Identification

- ID: TS-[DOMAIN]-[NUMBER]
- Titre: [Titre descriptif]
- Objectif: [Ce que le suite valide]
- Priorité: P0/P1/P2/P3

## Portée

- Domaine: [identity/account/billing/email/trust-risk/platform]
- Niveau: [unit/integration/e2e/smoke/performance/security]
- Objet de test: [Service ou composant]

## Cas de test

| ID     | Titre | Technique | Priorité | Automatisé |
| ------ | ----- | --------- | -------- | ---------- |
| TC-001 | ...   | ...       | P0       | Oui        |

## Critères de sortie

- [ ] Tous les cas P0 passent
- [ ] Coverage minimum atteint
- [ ] Pas de régression connue

## Environnement

- Commande: [commande d'exécution]
- Artefact: [format de sortie]
- Durée max: [timeout]
```

### 3.2 Mapping nvbes — Suites existantes

| Suite ID                | Catégorie manifeste | Commande                                                              |
| ----------------------- | ------------------- | --------------------------------------------------------------------- |
| TS-IDENTITY-AUTH        | `auth`              | `cargo test --package nvbes-identity-service identity.auth`           |
| TS-IDENTITY-MFA         | `security`          | `cargo test --package nvbes-identity-service identity.mfa`            |
| TS-IDENTITY-TOKENS      | `tokens`            | `cargo test --package nvbes-identity-service identity.tokens`         |
| TS-EMAIL-WEBHOOKS       | `webhook`           | `cargo test --package nvbes-email-worker email.worker.webhook`        |
| TS-EMAIL-DISPATCH       | `dispatch`          | `cargo test --package nvbes-email-worker email.worker.dispatcher`     |
| TS-BILLING-WEBHOOKS     | `webhook`           | `cargo test --package nvbes-billing-service billing.webhooks`         |
| TS-TRUST-RISK-ASSESS    | `assessment`        | `cargo test --package nvbes-trust-risk-service trust_risk.assessment` |
| TS-TRUST-RISK-INGESTION | `trust`             | `cargo test --package nvbes-trust-risk-service trust_risk.ingress`    |
| TS-TRUST-RISK-RULES     | `trust`             | `cargo test --package nvbes-trust-risk-service trust_risk.rules`      |
| TS-EMAIL-RETENTION      | `retention`         | `cargo test --package nvbes-email-worker email.worker.retention`      |
| TS-EMAIL-GRPC           | `grpc`              | `cargo test --package nvbes-email-worker email.worker.grpc`           |
| TS-IDENTITY-HEALTH      | `health`            | `cargo test --package nvbes-identity-service identity.health`         |
| TS-PLATFORM-COCKPIT     | `existence`         | `cargo test --package nvbes-platform platform.cockpit`                |
| TS-PLATFORM-FINOPS      | `existence`         | `cargo test --package nvbes-platform platform.finops`                 |
| TS-PLATFORM-AUDIT       | `existence`         | `cargo test --package nvbes-platform platform.audit`                  |
| TS-PLATFORM-OPS         | `security`          | `cargo test --package nvbes-platform platform.operations`             |
| TS-ACCOUNT-DB           | `database`          | `cargo test --package nvbes-account-service account.database`         |
| TS-CONTAINER-CONTRACT   | `containers`        | `node --test apps/*/tests/container-contract.test.mjs`                |
| TS-K6-LOAD              | `load`              | k6 `load` profile                                                     |
| TS-PLAYWRIGHT-E2E       | `end-to-end`        | `pnpm test:e2e:critical`                                              |

## 4. Test Case Specification

### 4.1 Template — Test Case

```markdown
# Test Case: [Titre]

## Identification

- ID: TC-[DOMAIN]-[NUMBER]
- Titre: [Titre descriptif]
- Suite: [ID de la suite parente]
- Priorité: P0/P1/P2/P3

## Conditions préalables

- [État du système avant exécution]
- [Données requises]
- [Environnement]

## Données d'entrée

| Paramètre | Valeur | Type |
| --------- | ------ | ---- |
| ...       | ...    | ...  |

## Étapes d'exécution

| #   | Action   | Résultat attendu |
| --- | -------- | ---------------- |
| 1   | [Action] | [Résultat]       |
| 2   | [Action] | [Résultat]       |

## Résultat attendu

[Description du résultat final attendu]

## Critères d'oracle

- [Comment vérifier le succès]
- [Comment détecter un échec]

## Données de test

[Fixtures, seeds, mocks requis]

## Environnement d'exécution

- Commande: [commande exacte]
- Timeout: [durée max]
```

### 4.2 Exemple — Test Case nvbes

```markdown
# Test Case: Validation token expiré rejeté

## Identification

- ID: TC-IDENTITY-AUTH-003
- Titre: Le service rejette un JWT expiré
- Suite: TS-IDENTITY-AUTH
- Priorité: P0

## Conditions préalables

- Service identity démarré
- Clé de signature JWT configurée

## Données d'entrée

| Paramètre | Valeur               | Type   |
| --------- | -------------------- | ------ |
| token     | JWT avec exp < now() | String |

## Étapes d'exécution

| #   | Action                                     | Résultat attendu           |
| --- | ------------------------------------------ | -------------------------- |
| 1   | Générer un JWT avec exp = now() - 3600     | Token signé                |
| 2   | Appeler GET /api/auth/me avec Bearer token | HTTP 401                   |
| 3   | Vérifier le body de réponse                | {"error": "token_expired"} |

## Résultat attendu

Le service retourne 401 Unauthorized avec un message "token_expired".

## Critères d'oracle

- Status code = 401
- Body contient "token_expired"
- Aucun log contenant de données personnelles

## Environnement d'exécution

- Commande: `cargo test --package nvbes-identity-service identity.auth::tests::test_expired_token_rejected`
- Timeout: 10s
```

## 5. Test Execution Report

### 5.1 Template

```markdown
# Test Execution Report — [Run ID]

## Environnement

- Date: [ISO 8601]
- Branch: [git branch]
- Commit: [SHA]
- Runner: [local/CI/staging]
- Lane: [PR/nightly/weekly/release]

## Résumé

| Métrique          | Valeur |
| ----------------- | ------ |
| Total tests       | [N]    |
| Passés            | [N]    |
| Échoués           | [N]    |
| Ignorés           | [N]    |
| Durée totale      | [s]    |
| Coverage lignes   | [%]    |
| Coverage branches | [%]    |

## Détails par lane

| Lane        | Statut | Durée | Tests   | Coverage |
| ----------- | ------ | ----- | ------- | -------- |
| Unit        | ✅/❌  | [s]   | [N]     | [%]      |
| Integration | ✅/❌  | [s]   | [N]     | [%]      |
| Contract    | ✅/❌  | [s]   | [N]     | —        |
| E2E         | ✅/❌  | [s]   | [N]     | —        |
| Smoke       | ✅/❌  | [s]   | [N]     | —        |
| k6          | ✅/❌  | [s]   | [iters] | —        |
| Security    | ✅/❌  | [s]   | [N]     | —        |

## Défaillances

| ID  | Test | Erreur | Sévérité | Ticket |
| --- | ---- | ------ | -------- | ------ |
| ... | ...  | ...    | ...      | ...    |

## Décision

- [ ] GO (tous les gates passent)
- [ ] NO-GO (gates échoués)

## Approbation

- Exécuté par: [nom/rôle]
- Date: [ISO 8601]
```

### 5.2 Mapping nvbes

| Section      | Source                         |
| ------------ | ------------------------------ |
| Lane status  | GitHub Actions job status      |
| Coverage     | `cargo llvm-cov` report        |
| k6 metrics   | k6 summary JSON                |
| Security     | OSV/cargo-deny/gitleaks output |
| Défaillances | CI logs + GitHub issues        |
| Decision     | CI gate result (ci-gate job)   |

## 6. Test Summary Report (Release)

### 6.1 Template

```markdown
# Test Summary Report — Release [VERSION]

## 1. Résumé exécutif

[Version testée, dates, verdict global]

## 2. Portée

[Services testés, lanes exécutées]

## 3. Résultats par catégorie

| Catégorie   | Lane           | Statut | Preuve     |
| ----------- | -------------- | ------ | ---------- |
| Smoke       | Post-deploy    | ✅/❌  | [artifact] |
| E2E         | Release gate   | ✅/❌  | [artifact] |
| Performance | k6 weekly      | ✅/❌  | [artifact] |
| Security    | check:security | ✅/❌  | [artifact] |

## 4. Couverture

[Coverage par crate, thresholds]

## 5. Défauts

[Bugs trouvés, corrigés, known issues]

## 6. Risques résiduels

[Risques acceptés, mitigations]

## 7. Recommandations

[Go/No-go, conditions]

## 8. Approbations

| Rôle           | Nom | Date | Signature |
| -------------- | --- | ---- | --------- |
| Test Manager   | ... | ...  | ...       |
| Security Owner | ... | ...  | ...       |
| Product Owner  | ... | ...  | ...       |
```

### 6.2 Génération

Le rapport est généré à partir des sources machine-readable du §8 :
manifeste de test Account, rapport `cargo llvm-cov` et seuils par crate.

```bash
# Tests du générateur
pnpm test:test-summary

# Génération (nécessite une couverture calculée au préalable)
pnpm test:rust:coverage
TSR_RELEASE=0.1.0-diag pnpm report:test-summary
# → .temp/test-summary/test-summary-report.md
```

La section « Résultats par catégorie » est dérivée du manifeste : chaque
catégorie couverte par un known gap est marquée `❌`. Le verdict global est
calculé par `deriveReleaseBlockingCategories` : `NO-GO` si au moins un gap
release-blocking existe, `GO` sinon. La section « Couverture » réutilise
`evaluateCoverage` du gate `check-coverage.mjs` pour comparer chaque crate à
son seuil. Le générateur vit dans `tools/test-summary/` et est testé par
`node --test` (cible Nx `test-summary:test`).

## 7. Acceptance Report (Signé)

### 7.1 Format

Le rapport d'acceptation suit le format défini dans
`docs/testing/account-acceptance-charters.md` § Paquet de preuve de production :

```json
{
  "schemaVersion": 1,
  "release": "<SHA immuable>",
  "environment": "staging",
  "issuedAt": "2025-01-01T00:00:00.000Z",
  "expiresAt": "2025-02-01T00:00:00.000Z",
  "signerKeyId": "<key-id>",
  "reports": [
    {
      "category": "acceptance-alpha",
      "decision": "passed",
      "signedBy": "<role>",
      "signedAt": "2025-01-01T00:00:00.000Z",
      "release": "<SHA>",
      "environment": "staging",
      "path": "reports/alpha.md",
      "sha256": "<hash>"
    }
  ],
  "deployment": {
    "path": "deployment.json",
    "sha256": "<hash>",
    "signature": "<detached-sig>",
    "signerKeyId": "<control-plane-key-id>"
  }
}
```

Signature :

Le paquet est signé avec la clé Ed25519 du release board par le script dédié
(`sign-acceptance-evidence.mjs`), hors dépôt. Il assemble l'enveloppe, calcule les
digests SHA-256 des rapports et de l'attestation de déploiement, puis signe les octets
exacts écrits (signature détachée 64 octets) :

```bash
pnpm sign:account-acceptance-evidence \
  --evidence-root <dossier-paquet> \
  --private-key account-acceptance-private.pem \
  --release <SHA-immuable> \
  --signer-key-id <key-id-release-board> \
  --deployment-key-id <control-plane-key-id> \
  --signed-by "Alpha Owner@acceptance-alpha" \
  --signed-by "Beta Owner@acceptance-beta" \
  # ... un --signed-by par catégorie humaine —
  --public-key account-acceptance-public.pem   # auto-vérification + empreinte
```

Génération d'une paire de clés et empreinte à piner dans l'Environment :

```bash
pnpm sign:account-acceptance-evidence generate-key .keys --name account-acceptance
```

Le script garantit les mêmes contraintes que le validateur : paquet staging lié au
release immuable, fraîcheur de chaque décision (31 j pentest, 14 j Beta, 7 j autres),
validité maximale 31 jours, chemin sûr, rapports réguliers non symboliques, et clé
privée en permissions 0600. La clé privée ne doit jamais entrer dans le dépôt ou un
runner de vérification. Équivalent manuel `openssl` (mêmes octets signés) :

```bash
openssl pkeyutl -sign -rawin \
  -inkey account-acceptance-private.pem \
  -in acceptance-evidence.json \
  -out acceptance-evidence.sig
```

## 8. Mapping machine-readable

### 8.1 Manifeste de test

Le fichier `docs/testing/account-test-manifest.json` (schemaVersion 4) est le manifeste
machine-readable qui mappe chaque catégorie ISO 29119 vers :

```json
{
  "category": "unit",
  "lane": "pullRequest",
  "execution": "automated",
  "command": "pnpm nx affected -t test",
  "artifact": "Nx and framework test reports",
  "iso29119Techniques": [
    "equivalence-partitioning",
    "boundary-value-analysis",
    "statement-coverage",
    "branch-coverage",
    "error-guessing"
  ]
}
```

Chaque entrée du manifeste inclut :

- `category` : identifiant ISO 29119 aligné
- `lane` : CI lane correspondante
- `execution` : `automated`, `manual-command`, `human`, `gap`
- `command` : commande d'exécution (si automatisée ou manual-command)
- `artifact` : artefact de preuve produit
- `iso29119Techniques` : techniques ISO 29119-4 appliquées (non vide)
- `limitation` : contraintes connues (optionnel)
- `blocker` : cause du gap (si execution=gap)
- `runbook` : lien runbook (si execution=human)

Le catalogue de techniques valides est défini dans `check-test-manifest.mjs`
(`validIso29119Techniques`) :

```
equivalence-partitioning, boundary-value-analysis, state-transition,
cause-effect, decision-table, use-case, error-guessing, risk-based,
exploratory-testing, statement-coverage, branch-coverage,
condition-coverage, path-coverage, basis-path-coverage, mutation-testing
```

### 8.2 Validation du manifeste

```bash
# Valider la structure du manifeste
pnpm nx run account-quality:test

# Vérifier les blocking categories
pnpm check:account-release-readiness
```

Le validator refuse :

- Catégorie manquante ou dupliquée
- Commande automatisée sans artefact
- Cron divergent du workflow
- Fausse déclaration d'automatisation
- Sérialisation de secrets dans les artefacts
- `iso29119Techniques` manquant ouvide sur une catégorie
- Technique ISO 29119-4 inconnue dans le catalogue valide
- `schemaVersion` différent de 4
