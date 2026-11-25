# Stratégie de test Account

> **Statut : socle V1 actif.** Le runtime propriétaire est
> `apps/account-service`. Les applications historiques `account-web` et
> `account-worker` sont archivées et hors preuve V1. Toute campagne suit la
> [direction produit](../product/nvbes-product-strategy.md) et le budget
> 30 EUR TTC.

## Portée et statut de preuve

Cette stratégie couvre :

- `libs/rust/account` monté par `apps/api` (profil, préférences, consentements,
  export, fermeture) et la tâche worker `privacy-retention` ;
- `libs/ts/account-sdk-core` (contrat OpenAPI + types générés) ;
- les événements versionnés `contracts/events/account.*.v1.schema.json`.

La source de vérité machine-readable V1 est
[`docs/testing/v1/account.json`](v1/account.json). Le manifeste historique
`account-test-manifest.json` et le harness `tools/account-quality` restent
diagnostiques pour des campagnes archivées ; ils ne prononcent plus GO.

## Surface HTTP V1

| Route                               | Rôle                        |
| ----------------------------------- | --------------------------- |
| `GET/PUT /api/v1/profile`           | profil Account              |
| `GET/PUT /api/v1/preferences`       | thème / langue              |
| `GET/PUT /api/v1/notifications`     | préférences de notification |
| `GET/POST/DELETE /api/v1/consents`  | acceptations légales        |
| `POST/GET /api/v1/privacy/exports*` | export RGPD                 |
| `GET/POST /api/v1/closure*`         | fermeture / annulation      |
| `GET /health/live`, `GET /metrics`  | ops                         |

Audience JWT : `nvbes-account-service`. Scopes :
`account:read|write|export|close`.

## État d'automatisation réel

| Lane             | Déclenchement                     | Couverture                                                                 | Preuve                          |
| ---------------- | --------------------------------- | -------------------------------------------------------------------------- | ------------------------------- |
| PR               | `account-service:check` / `:test` | compilation, auth step-up, consentements, config fail-closed               | Cargo / nextest                 |
| Database         | `account-service:test:database`   | cycle de vie (profil, consentements, export, fermeture, audit append-only) | PostgreSQL isolé `account_test` |
| Contract         | `account-service:test:contract`   | images API et worker non-root, `serve` / `run <task>`                      | `container-contract.test.mjs`   |
| OpenAPI          | `account-sdk-core:check`          | paths V1 + scopes + sync `types.gen.ts`                                    | `openapi.contract.test.mjs`     |
| Trusted hermetic | opérateur                         | `nvbes-worker run migrate`, `nvbes-worker run privacy-retention`           | journaux smoke                  |
| Pre-release      | session signée                    | acceptation opérateur runbook Platform Operations                          | preuve manuelle                 |

### Limites explicites

- Pas d'UI Account dans la V1 ; pas de Playwright Account.
- Pas d'équipes ni de RLS multi-tenant en V1 : isolation par `principal_id`.
- Pas d'outbox Account en V1 : aucun consommateur ; l'audit append-only
  trace chaque mutation.
- `account-web` / `account-worker` / DAST/k6 historiques ne comptent pas comme
  preuves du runtime lean.

## Fiabilité du harness

- Les tests DB refusent les hôtes non loopback / non CI et exigent un nom de
  base contenant `account_test`.
- Les smokes synthétiques n'utilisent que des UUID de test.
- Fermeture et export exigent step-up (`totp` / `webauthn`).
- Audit append-only ; mutations audit rejetées.

## Gates

- PR : check, unitaires, contrat conteneur, OpenAPI Account.
- Avant staging : `nvbes-worker run migrate` et `run privacy-retention` verts.
- Avant production : preuves `docs/testing/v1/account.json` + acceptation
  opérateur ; aucun GO via le manifeste historique.
