# ISO/IEC/IEEE 29119-5 — Tests pilotés par mots-clés

> Framework de keyword-driven testing pour l'automatisation nvbes.

## 1. Concepts

### 1.1 Définition ISO 29119-5

Le keyword-driven testing sépare la conception des tests (mots-clés
abstraits) de leur exécution (scripts concrets). Chaque mot-clé représente
une action métier réutilisable.

### 1.2 Architecture

```
┌─────────────────────────────────────────────────────┐
│                   Test Cases                         │
│  (fichiers YAML/JSON déclarant des séquences        │
│   de mots-clés avec des paramètres)                 │
├─────────────────────────────────────────────────────┤
│                Keyword Registry                      │
│  (bibliothèque centralisée de mots-clés             │
│   avec leur implémentation)                         │
├─────────────────────────────────────────────────────┤
│               Keyword Executors                      │
│  (Rust unit/integration, Playwright E2E,            │
│   k6 performance, shell scripts)                    │
├─────────────────────────────────────────────────────┤
│              Test Infrastructure                     │
│  (DB, HTTP clients, Docker, staging)                │
└─────────────────────────────────────────────────────┘
```

## 2. Registre de mots-clés

La source de vérité des mots-clés est le registre implémenté dans
`libs/rust/test-utils/src/test.runner.keywords.*.rs` (dont les mots-clés
Rust d'exécution ci-dessous). Les exemples de code des sections 3 et 4 sont
des illustrations de patterns, pas des contrats : seuls les mots-clés
enregistrés et les routes réelles décrites ci-dessous font foi.

### 2.1 Mots-clés Identity

Le service Identity est un runtime fermé : **aucune route HTTP publique
d'auth n'existe** (le contrat conteneur asserte l'absence de
`/api/auth/*`). Sa preuve de comportement passe par les smokes CLI
(`synthetic-auth-smoke`, `synthetic-mfa-smoke`, `synthetic-token-smoke`,
`validate-runtime`) et par les tests Rust ciblés. Le mot-clé d'exécution
couvre uniquement la surface de santé réelle.

| Mot-clé                 | Paramètres        | Implémentation          | Fichier source   |
| ----------------------- | ----------------- | ----------------------- | ---------------- |
| `identity.health_check` | [expected_status] | HTTP GET `/health/live` | `identity_kw.rs` |

- Port par défaut : `127.0.0.1:3060` (`NVBES_IDENTITY_BASE_URL`).
- L'authentification de bout en bout est exercée par les suites Rust/CLI de
  `identity-test-strategy.md`, pas par des mots-clés HTTP d'auth.

### 2.2 Mots-clés Billing

La route webhook réelle est `POST /webhooks/stripe`, authentifiée par le
schéma de signature Stripe `Stripe-Signature: t=<ts>,v1=<hex>` (HMAC-SHA256
sur `"{timestamp}.{payload}"`, tolérance 300 s, clavier normalisé). Le
service rejette les événements `livemode: true` et les signatures invalides
ou périmées ; il n'exige pas de Bearer token pour la livraison webhook.

| Mot-clé                     | Paramètres                                        | Implémentation                                                                                   | Fichier source  |
| --------------------------- | ------------------------------------------------- | ------------------------------------------------------------------------------------------------ | --------------- |
| `billing.create_webhook`    | event_type, [payload], [livemode=false], [secret] | Génère `evt_test_*` et signe offline → `event`, `event_payload`, `signature`, `timestamp`        | `billing_kw.rs` |
| `billing.verify_signature`  | payload, signature, secret                        | Vérification offline HMAC-SHA256 + fraîcheur → `valid`, `stale`, `expected_signature`            | `billing_kw.rs` |
| `billing.process_event`     | event \| event_payload, [signature], [secret]     | HTTP POST `/webhooks/stripe` + header `Stripe-Signature` → `delivery_1_status`, `processed`      | `billing_kw.rs` |
| `billing.check_idempotence` | event \| event_payload, [signature], [secret]     | Même événement signé délivré 2× → `delivery_1_status`, `delivery_2_status`, `idempotent_handled` | `billing_kw.rs` |

- Port par défaut : `127.0.0.1:3002` (`NVBES_BILLING_BASE_URL`).
- Secret par défaut : `whsec_test_secret` (`NVBES_BILLING_WEBHOOK_SECRET`),
  cohérent avec l'environnement de test du service ; signer avec le même
  secret que celui configuré côté service.
- `create_webhook` est offline (pas de réseau) : il peut servir dans les
  suites unitaires aussi bien que dans la série de corridors d'intégration.

### 2.3 Mots-clés Email

Ingress producteur = gRPC `SubmitEmail`. La surface HTTP réelle est : route
d'acceptation `POST /internal/queue/email-dispatch` (corps brut = `message_id`
UUID, `204 Acknowledged` / `503 Retry`), webhook provider
`POST /webhooks/scaleway/topics-and-events` (JSON SNS, `Content-Type:
application/json`), observabilité `GET /metrics` (compteur
`email_messages_total`). Le replay durable est une opération gRPC, hors
périmètre des mots-clés HTTP.

| Mot-clé                 | Paramètres                                         | Implémentation                                                                                      | Fichier source |
| ----------------------- | -------------------------------------------------- | --------------------------------------------------------------------------------------------------- | -------------- |
| `email.send`            | message_id                                         | HTTP POST `/internal/queue/email-dispatch` (corps = UUID) → `status`, `send` (204)                  | `email_kw.rs`  |
| `email.retry_failed`    | message_id                                         | Re-dispatch HTTP de l'UUID sur `/internal/queue/email-dispatch` → `retry_failed` (204)              | `email_kw.rs`  |
| `email.capture_webhook` | message, [message_type=Notification], [topic]      | HTTP POST `/webhooks/scaleway/topics-and-events` → `status`, `kind`                                 | `email_kw.rs`  |
| `email.verify_delivery` | [expected_messages=1], [timeout_ms], [interval_ms] | Polling `GET /metrics` sur `email_messages_total` → `delivered`, `messages_total`, `webhooks_total` | `email_kw.rs`  |

- Port par défaut : `127.0.0.1:3040` (`NVBES_EMAIL_BASE_URL`).
- Tokens optionnels : `NVBES_EMAIL_INTERNAL_TOKEN` (routes `/internal/*`),
  `NVBES_EMAIL_METRICS_TOKEN` (Bearer pour `/metrics`).
- Acceptation = « commande durablement acceptée » : le keyword `email.send`
  reflète l'ack de la file, pas la remise par le provider (voir le contrat
  at-least-once dans `email-test-strategy.md`).

### 2.4 Mots-clés Trust/Risk

Le service Trust/Risk n'expose **aucune route HTTP métier** : les mots-clés
`trust.*` passent par l'adaptateur JSON→proto du runner et le client gRPC réels
(`TrustRiskClient` dans `libs/rust/trust-risk`), contre les services gRPC listés
dans `trust-risk-test-strategy.md`. Aucun endpoint HTTP fictif n'est introduit.

| Mot-clé                | Paramètres                                                                      | Implémentation                                                                               | Fichier source     |
| ---------------------- | ------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- | ------------------ |
| `trust.submit_signals` | signals \| signal fields, [auth_token], [endpoint], [environment], [timeout_ms] | gRPC `TrustRiskSignalService/SubmitSignals` → `status`, `accepted`, `duplicates`, `receipts` | `trust_risk_kw.rs` |
| `trust.assess_risk`    | producer, assessment_key, operation_class, [subjects], [signals], [context]     | gRPC `TrustRiskAssessmentService/AssessRisk` → `score`, `band`, `recommendation`, `reasons`  | `trust_risk_kw.rs` |
| `trust.submit_labels`  | labels, [auth_token], [endpoint], [environment], [timeout_ms]                   | gRPC `TrustRiskLabelService/SubmitLabels` → `status`, `accepted`, `duplicates`, `receipts`   | `trust_risk_kw.rs` |

- Endpoint par défaut : `http://127.0.0.1:3050` (`NVBES_TRUST_RISK_GRPC_ENDPOINT`).
- Token requise : `auth_token` ou `NVBES_TRUST_RISK_GRPC_AUTH_TOKEN` (min. 32
  caractères, cohérent avec `TrustRiskClientConfig`).
- Environnement : `environment` ou `NVBES_ENV`, défaut `test` (https requis hors
  development/test).
- Un signal peut être décrit soit par un tableau `signals`, soit par des champs
  de premier niveau (`producer`, `signal_kind`, `subjects`, `attributes`,
  `scope`…). Les enums de proto acceptent les noms (`tenant`, `principal`,
  `confirmed_fraud`…) ou les valeurs numériques.

### 2.5 Mots-clés Infrastructure

| Mot-clé               | Paramètres               | Implémentation                       | Fichier source |
| --------------------- | ------------------------ | ------------------------------------ | -------------- |
| `infra.setup_db`      | database_name            | psql CREATE DATABASE éphémère        | `infra_kw.rs`  |
| `infra.migrate`       | direction, database_url  | sqlx migrate run/revert              | `infra_kw.rs`  |
| `infra.start_service` | service_name, health_url | tokio::process Command + health poll | `infra_kw.rs`  |
| `infra.health_check`  | url, expected_status     | HTTP GET + status validation         | `infra_kw.rs`  |
| `infra.cleanup`       | resources, database_url  | psql DROP DATABASE WITH FORCE        | `infra_kw.rs`  |

### 2.6 Mots-clés Playwright (E2E)

| Mot-clé                 | Paramètres      | Implémentation                | Fichier source  |
| ----------------------- | --------------- | ----------------------------- | --------------- |
| `e2e.navigate`          | url             | page.goto()                   | Playwright spec |
| `e2e.fill`              | selector, value | page.fill()                   | Playwright spec |
| `e2e.click`             | selector        | page.click()                  | Playwright spec |
| `e2e.assert_text`       | selector, text  | expect(locator).toHaveText()  | Playwright spec |
| `e2e.assert_visible`    | selector        | expect(locator).toBeVisible() | Playwright spec |
| `e2e.wait_for_response` | url_pattern     | page.waitForResponse()        | Playwright spec |
| `e2e.screenshot`        | name            | page.screenshot()             | Playwright spec |

## 3. Format de test case YAML

### 3.1 Structure

```yaml
# Test Case: [Titre]
id: TC-[DOMAIN]-[NUMBER]
priority: P0/P1/P2/P3
tags: [unit, integration, e2e, smoke, security, performance]
setup:
  - keyword: infra.setup_db
    params:
      name: test_db
  - keyword: infra.migrate
    params:
      direction: up

steps:
  - keyword: identity.create_account
    params:
      email: 'test-${UNIQUE}@example.com'
      password: 'SecureP@ss123!'
    expect:
      status: 201
      body:
        id: '{{uuid}}'

  - keyword: identity.verify_email
    params:
      token: '${LAST.body.verify_token}'
    expect:
      status: 200

  - keyword: identity.login
    params:
      email: '${SETUP.email}'
      password: '${SETUP.password}'
    expect:
      status: 200
      body:
        session_token: '{{string}}'

teardown:
  - keyword: infra.cleanup
    params:
      resources: '${SETUP.resources}'
```

> Exemple illustratif : `identity.create_account`/`identity.login` ne sont
> **pas** des mots-clés enregistrés (Identity n'a pas de route HTTP d'auth).
> Les mots-clés réellement exécutables sont ceux des tables §2 ; un test case
> dont le mot-clé n'est pas dans le registre est refusé (`keyword not found`).

### 3.2 Variables et substitutions

| Syntaxe                 | Description                   |
| ----------------------- | ----------------------------- |
| `${VAR}`                | Variable d'environnement      |
| `${SETUP.param}`        | Paramètre de setup            |
| `${LAST.body.field}`    | Champs de la dernière réponse |
| `${STEP[n].body.field}` | Champs d'un step spécifique   |
| `{{uuid}}`              | UUID généré automatiquement   |
| `{{timestamp}}`         | Timestamp ISO 8601            |
| `{{unique}}`            | String unique (nanoid)        |

### 3.3 Assertions

```yaml
# Assertion exacte
expect:
  status: 200

# Assertion partielle
expect:
  body:
    token: '{{string}}'
    expires_at: '{{ISO8601}}'

# Assertion regex
expect:
  body:
    email: "{{regex:user@.+\\.com}}"

# Assertion négative
expect:
  status: 401
  body:
    error: 'unauthorized'

# Assertion de timing
expect:
  duration_ms: '<500'
```

## 4. Implémentation par niveau

### 4.1 Niveau unitaire (Rust)

Les mots-clés s'implémentent comme des fonctions Rust réutilisables :

```rust
// libs/rust/core/src/testing/keywords/identity.rs

pub async fn create_account(
    db: &PgPool,
    email: &str,
    password: &str,
) -> Result<Account, KeywordError> {
    let hashed = hash_password(password).await?;
    let account = sqlx::query_as!(
        Account,
        r#"INSERT INTO accounts (email, password_hash) VALUES ($1, $2) RETURNING *"#,
        email,
        hashed,
    )
    .fetch_one(db)
    .await?;
    Ok(account)
}

pub async fn verify_email(
    db: &PgPool,
    token: &str,
) -> Result<(), KeywordError> {
    let record = sqlx::query!(
        r#"SELECT account_id FROM email_verifications WHERE token = $1 AND used = false"#,
        token,
    )
    .fetch_optional(db)
    .await?
    .ok_or(KeywordError::InvalidToken)?;

    sqlx::query!(
        r#"UPDATE accounts SET email_verified = true WHERE id = $1"#,
        record.account_id,
    )
    .execute(db)
    .await?;

    Ok(())
}

pub async fn login(
    db: &PgPool,
    email: &str,
    password: &str,
) -> Result<Session, KeywordError> {
    let account = find_account_by_email(db, email).await?;
    verify_password(&account.password_hash, password).await?;
    let session = create_session(db, account.id).await?;
    Ok(session)
}
```

### 4.2 Niveau intégration

Les mots-clés d'intégration combinent plusieurs actions :

```rust
// Test d'intégration utilisant les mots-clés
#[tokio::test]
async fn test_full_registration_flow() {
    let db = setup_test_db().await;

    // Step 1: Create account
    let account = create_account(&db, "test@example.com", "P@ss123!")
        .await.unwrap();
    assert!(!account.email_verified);

    // Step 2: Verify email
    let token = get_verification_token(&db, account.id).await.unwrap();
    verify_email(&db, &token).await.unwrap();

    // Step 3: Login
    let session = login(&db, "test@example.com", "P@ss123!").await.unwrap();
    assert!(!session.token.is_empty());

    // Step 4: Revoke session
    revoke_session(&db, session.id).await.unwrap();
    assert!(is_session_revoked(&db, session.id).await.unwrap());

    cleanup_test_db(&db).await;
}
```

### 4.3 Niveau E2E (Playwright)

Les mots-clés E2E映射é vers des helpers Playwright :

```typescript
// tests/e2e/keywords/identity.ts
import { Page, expect } from '@playwright/test';

export async function navigateToRegister(page: Page, baseUrl: string) {
  await page.goto(`${baseUrl}/register`);
  await expect(page.getByRole('heading', { name: /create account/i })).toBeVisible();
}

export async function fillRegistrationForm(page: Page, email: string, password: string) {
  await page.getByLabel(/email/i).fill(email);
  await page.getByLabel(/password/i).fill(password);
  await page.getByRole('button', { name: /create account/i }).click();
}

export async function verifyEmailConfirmation(page: Page) {
  await expect(page.getByText(/check your email/i)).toBeVisible();
}

export async function login(page: Page, baseUrl: string, email: string, password: string) {
  await page.goto(`${baseUrl}/login`);
  await page.getByLabel(/email/i).fill(email);
  await page.getByLabel(/password/i).fill(password);
  await page.getByRole('button', { name: /sign in/i }).click();
  await expect(page.getByRole('heading', { name: /dashboard/i })).toBeVisible();
}
```

### 4.4 Niveau performance (k6)

Les mots-clés k6映射é vers des scenarios :

```javascript
// tools/load-tests/account/keywords/api.js
import http from 'k6/http';
import { check } from 'k6';

export function createAccount(baseUrl, email, password) {
  const res = http.post(
    `${baseUrl}/api/auth/register`,
    JSON.stringify({
      email,
      password,
    }),
    {
      headers: { 'Content-Type': 'application/json' },
    },
  );

  check(res, {
    'account created': (r) => r.status === 201,
    'has account id': (r) => JSON.parse(r.body).id !== undefined,
  });

  return res;
}

export function loginApi(baseUrl, email, password) {
  const res = http.post(
    `${baseUrl}/api/auth/login`,
    JSON.stringify({
      email,
      password,
    }),
    {
      headers: { 'Content-Type': 'application/json' },
    },
  );

  check(res, {
    'login successful': (r) => r.status === 200,
    'has session token': (r) => JSON.parse(r.body).session_token !== undefined,
  });

  return res;
}

export function apiHealthCheck(baseUrl) {
  const res = http.get(`${baseUrl}/health`);
  check(res, {
    'health ok': (r) => r.status === 200,
    'has release sha': (r) => JSON.parse(r.body).release_id !== undefined,
  });
  return res;
}
```

## 5. Exécution des tests keyword-driven

### 5.1 Par niveau

| Niveau      | Runner     | Commande                  | Format           |
| ----------- | ---------- | ------------------------- | ---------------- |
| Unit        | cargo test | `cargo test --lib --bins` | Rust inline      |
| Integration | cargo test | `cargo test --tests`      | Rust inline      |
| E2E         | Playwright | `pnpm test:e2e:critical`  | TypeScript specs |
| Performance | k6         | k6 scripts                | JavaScript       |
| Contract    | node:test  | `node --test *.test.mjs`  | ESM modules      |

### 5.2 Orchestration

```bash
# Exécution complète par niveau
pnpm test:unit           # Unit (Rust + web typecheck)
pnpm test:integration    # Integration (Rust + Terraform)
pnpm test:e2e:critical   # E2E (Playwright)
pnpm test:smoke          # Smoke (HTTP)
pnpm check:security      # Sécurité complète

# Exécution par service
cargo test --package nvbes-identity-service
cargo test --package nvbes-email-worker
cargo test --package nvbes-billing-service
cargo test --package nvbes-trust-risk-service
cargo test --package nvbes-account-service

# Coverage
pnpm test:rust:coverage
```

## 5.3 Validation contre le manifeste JSON

Le runner valide chaque test case contre le manifeste de test avant exécution :

```bash
nvbes-runner test-cases/billing-001.yaml --manifest docs/testing/account-test-manifest.json
```

Contrôles effectués :

| Contrôle         | Description                                                                        |
| ---------------- | ---------------------------------------------------------------------------------- |
| Keywords connus  | Chaque mot-clé du test case doit être enregistré dans le registry                  |
| Schema version   | Le manifeste doit être en version 4 (défaut)                                       |
| Tags vs coverage | Les tags du test case doivent correspondre aux catégories de coverage du manifeste |
| Gaps bloquants   | Un test case taggé sur une coverage `gap` bloquante est refusé                     |
| Politique        | Vérifie `silentSkipsAllowed=false` et `retryGreenAllowed=false`                    |

## 6. Avantages du modèle keyword-driven pour nvbes

| Avantage                 | Impact                                                                     |
| ------------------------ | -------------------------------------------------------------------------- |
| Réutilisabilité          | Mêmes mots-clés (create_account, login) utilisés en unit, integration, E2E |
| Maintenabilité           | Changement d'implémentation = 1 fonction, pas 10 tests                     |
| Lisibilité               | Tests lisibles par non-développeurs (Product, QA)                          |
| Couverture multi-niveaux | Même scénario testé en unit → integration → E2E                            |
| Documentation vivante    | Les mots-clés documentent les capacités du système                         |
| CI/CD intégré            | Mots-clés = fonctions Rust/TS = exécutables via cargo/npm                  |

## 7. Gaps et roadmap

| Gap                                     | Statut                                                                                                                          | Priorité    |
| --------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- | ----------- |
| Registre centralisé de mots-clés Rust   | **Implémenté** (37 keywords dans `libs/rust/test-utils`, sur les surfaces HTTP, de santé et gRPC réelles)                       | ~~P1~~ Done |
| Fichiers YAML de test cases déclaratifs | Non implémenté                                                                                                                  | P2          |
| Runner YAML → Rust/Playwright/k6        | Runner Rust présent ; intégrations navigateur/charge non démontrées                                                             | P2          |
| Mots-clés Trust/Risk (gRPC)             | **Implémenté** (adaptateur JSON→proto + `TrustRiskClient` — `trust.submit_signals`, `trust.assess_risk`, `trust.submit_labels`) | ~~P2~~ Done |
| Mots-clés Platform Operations           | **Implémenté** (6 keywords dans platform_kw)                                                                                    | ~~P2~~ Done |
| Intégration avec le manifeste JSON      | Ancien schema Account seulement ; migration au manifeste V1 requise                                                             | P1          |
| Mots-clés Billing (contrat Stripe réel) | **Implémenté** (create_webhook, verify_signature, process_event, check_idempotence sur `/webhooks/stripe`, schéma `t=,v1=`)     | ~~P1~~ Done |
| Mots-clés Email (surfaces réelles)      | Adaptateurs présents ; une métrique globale ne prouve pas la livraison du message du scénario                                   | P1          |
| Mots-clés Infrastructure                | **Implémenté** (setup_db, migrate, start_service, health_check, cleanup)                                                        | ~~P2~~ Done |

La roadmap privilégie d'abord l'extraction des mots-clés existants en
bibliothèque réutilisable, puis l'ajout d'un runner YAML déclaratif.

La présence d'un mot-clé ne prouve aucune exécution métier. Les exemples de
ce document sont des conceptions, pas des résultats du candidat. Les
schémas complets de paramètres, les assertions métier, les interruptions et
les scénarios intégrés restent à valider ; voir [l'état V1](../v1/README.md).
