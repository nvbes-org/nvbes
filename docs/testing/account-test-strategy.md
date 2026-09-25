# Stratégie de test Account

> **Statut : socle V1 actif.** Le runtime propriétaire est
> `apps/account-service`. Les applications historiques `account-web` et
> `account-worker` sont archivées et hors preuve V1. Toute campagne suit la
> [direction produit](../product/nvbes-product-strategy.md) et le budget
> 30 EUR TTC.

## Portée et statut de preuve

Cette stratégie couvre :

- `apps/account-service` (profil, préférences, consents, équipes, export,
  fermeture, outbox) ;
- `libs/ts/account-sdk-core` (contrat OpenAPI + types générés) ;
- les événements versionnés `contracts/events/account.*.v1.schema.json`.

La source de vérité machine-readable V1 est
[`docs/testing/v1/account.json`](v1/account.json). Le manifeste historique
`account-test-manifest.json` et le harness `tools/account-quality` restent
diagnostiques pour des campagnes archivées ; ils ne prononcent plus GO.

## Surface HTTP V1

| Route                                           | Rôle                        |
| ----------------------------------------------- | --------------------------- |
| `GET/PUT /api/v1/profile`                       | profil Account              |
| `GET/PUT /api/v1/preferences`                   | thème / langue              |
| `GET/PUT /api/v1/notifications`                 | préférences de notification |
| `GET/POST/DELETE /api/v1/consents`              | acceptations légales        |
| `GET/POST /api/v1/teams`                        | list / create               |
| `POST /api/v1/teams/join`                       | rejoindre                   |
| `GET /api/v1/teams/{id}`                        | détail                      |
| `GET /api/v1/teams/{id}/members`                | membres                     |
| `DELETE /api/v1/teams/{id}/members/{principal}` | retrait (owner)             |
| `POST /api/v1/teams/{id}/leave`                 | quitter                     |
| `POST/GET /api/v1/privacy/exports*`             | export RGPD                 |
| `GET/POST /api/v1/closure*`                     | fermeture / annulation      |
| `GET /health/live`, `GET /metrics`              | ops                         |

Audience JWT : `nvbes-account-service`. Scopes :
`account:read|write|export|close`.

## État d'automatisation réel

| Lane             | Déclenchement                     | Couverture                                                                     | Preuve                          |
| ---------------- | --------------------------------- | ------------------------------------------------------------------------------ | ------------------------------- |
| PR               | `account-service:check` / `:test` | compilation, auth step-up, consents, CLI maintenance                           | Cargo / nextest                 |
| Database         | `account-service:test:database`   | synthetic lifecycle (teams leave, consents, export, closure, outbox publish)   | PostgreSQL isolé `account_test` |
| Contract         | `account-service:test:contract`   | Dockerfile non-root, migrate/privacy/outbox CLI, step-up                       | `container-contract.test.mjs`   |
| OpenAPI          | `account-sdk-core:check`          | paths V1 + scopes + sync `types.gen.ts`                                        | `openapi.contract.test.mjs`     |
| Trusted hermetic | opérateur                         | `migrate`, `synthetic-account-smoke`, `process-privacy-jobs`, `publish-outbox` | journaux smoke                  |
| Pre-release      | session signée                    | acceptation opérateur runbook Platform Operations                              | preuve manuelle                 |

### Limites explicites

- Pas d'UI Account dans la V1 ; pas de Playwright Account.
- Pas de RLS multi-tenant Enterprise : isolation par `principal_id` et rôles
  d'équipe `owner`/`member`.
- L'outbox marque `published_at` sans sink externe tant qu'aucun consommateur
  n'est branché.
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
- Avant staging : `synthetic-account-smoke` + `publish-outbox` verts.
- Avant production : preuves `docs/testing/v1/account.json` + acceptation
  opérateur ; aucun GO via le manifeste historique.
