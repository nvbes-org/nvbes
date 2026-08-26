# Beta Readiness - nvbes Drive

## Statut

**Checklist future et inactive.** Aucune beta Drive n'est planifiée dans la V1
du socle. Réutiliser cette checklist uniquement si Cloud/Drive est sélectionné
ultérieurement comme produit, après une nouvelle revue produit, FinOps et
opérationnelle. Voir la
[direction V1 canonique](nvbes-product-strategy.md).

## Objectif

Preparer une beta fermee sur staging pour valider les parcours critiques avant ouverture payante.

La beta ne doit pas demarrer tant que les points `Go/No-Go` ne sont pas verts ou explicitement acceptes par les owners produit, tech, security/privacy et billing.

## Checklist de Lancement

### Produit

- [ ] Scope beta ferme: equipes pilotes, nombre de workspaces, duree, canal support.
- [ ] Parcours critiques verifies sur staging:
  - signup;
  - verification email;
  - login/logout;
  - creation workspace;
  - creation dossier;
  - upload metadata + activation;
  - lien public avec expiration;
  - revocation lien;
  - invitation membre;
  - quota visible;
  - billing overview visible;
  - verification que la creation de cle API legacy est refusee et que les integrations machine passent par service accounts OAuth.
- [ ] Etats critiques revus dans l'UI: vide, permission refusee, quota atteint, lien expire/revoque, billing bloque.
- [ ] Copy beta et messages d'erreur relus.
- [ ] Process support beta defini: canal, SLA beta, triage bug.

### Technique

- [ ] Staging deploye avec API, worker, web, PostgreSQL et bucket separes.
- [ ] Migrations appliquees via `pnpm db:migrate:staging`.
- [ ] Workers Identity et Drive lances via `pnpm dev:account-worker` / `pnpm dev:cloud-worker` ou leurs unites de deploiement.
- [ ] Identite synthetique staging provisionnee par le control plane avec un
      audit lie au release; aucun script local ne recoit un acces direct a la
      base ou a Redis staging.
- [ ] `pnpm release:gate:staging` vert avec URLs staging.
- [ ] `pnpm test:smoke:staging` vert apres deploiement.
- [ ] Logs JSON actifs en staging.
- [ ] Dashboards `/observability/dashboards` visibles.
- [ ] Alertes critiques `/observability/alerts/critical` visibles.
- [ ] Aucun bucket public.
- [ ] Sauvegarde staging recente ou snapshot restore point confirme avant migration.

### Security, Privacy, Compliance

- [ ] Verification email obligatoire avant usage workspace.
- [ ] Tokens de partage et API keys legacy hashes en base.
- [ ] Object keys absentes des reponses API, URLs signees et logs.
- [ ] Audit append-only actif.
- [ ] Acces audit limite Owner/Admin.
- [ ] Procedures RGPD export/suppression testees au moins en smoke manuel.
- [ ] Runbooks incidents disponibles: API, PostgreSQL, Object Storage, bucket public, billing webhook, jobs RGPD.
- [ ] Registres compliance initialises: traitements, violations, sous-traitants, retention.
- [ ] Sous-traitants staging valides ou marques comme placeholders.

### Billing et FinOps

- [ ] Stripe staging/test configure.
- [ ] `stripe_price_mappings` renseignes pour `solo_pro`, `team`, `team_plus`.
- [ ] `pnpm check:stripe-mappings` vert sur staging avant checkout beta.
- [ ] Webhook secret staging configure.
- [ ] Checkout test execute au moins une fois.
- [ ] Customer Portal test execute au moins une fois.
- [ ] Estimation facture visible dans l'API.
- [ ] Budget cloud staging active ou suivi manuel documente.

## Variables d'Environnement

Voir aussi `.env.example`.

### API Runtime

| Variable                                  | Obligatoire | Environnement         | Description                               |
| ----------------------------------------- | ----------- | --------------------- | ----------------------------------------- |
| `NVBES_APP_NAME`                        | Non         | Tous                  | Nom affiche par `/health`.                |
| `NVBES_ENV`                             | Oui         | Tous                  | `development`, `staging` ou `production`. |
| `NVBES_API_PORT`                        | Oui         | API                   | Port HTTP API.                            |
| `NVBES_DATABASE_URL`                    | Oui         | API/worker/migrations | URL PostgreSQL privee.                    |
| `NVBES_DATABASE_MAX_CONNECTIONS`        | Non         | API/worker            | Taille max pool SQLx.                     |
| `NVBES_AUTH_SESSION_TTL_HOURS`          | Non         | API                   | TTL sessions.                             |
| `NVBES_AUTH_VERIFICATION_TTL_HOURS`     | Non         | API                   | TTL token verification email.             |
| `NVBES_AUTH_PASSWORD_RESET_TTL_MINUTES` | Non         | API                   | TTL reset password.                       |

### Billing

| Variable                            | Obligatoire      | Environnement | Description                       |
| ----------------------------------- | ---------------- | ------------- | --------------------------------- |
| `NVBES_STRIPE_SECRET_KEY`         | Oui beta billing | API           | Cle secrete Stripe test/staging.  |
| `NVBES_STRIPE_WEBHOOK_SECRET`     | Oui beta billing | API           | Secret signature webhook Stripe.  |
| `NVBES_STRIPE_API_BASE_URL`       | Non              | API           | Base API Stripe, defaut officiel. |
| `NVBES_BILLING_SUCCESS_URL`       | Oui              | API           | Redirection checkout success.     |
| `NVBES_BILLING_CANCEL_URL`        | Oui              | API           | Redirection checkout cancel.      |
| `NVBES_BILLING_PORTAL_RETURN_URL` | Oui              | API           | Retour Customer Portal.           |

### Tests et Release

| Variable                          | Obligatoire | Usage                                                                  |
| --------------------------------- | ----------- | ---------------------------------------------------------------------- |
| `NVBES_WEB_BASE_URL`            | Oui         | `pnpm test:smoke`, `pnpm test:e2e:critical`.                           |
| `NVBES_API_BASE_URL`            | Oui         | `pnpm test:smoke`, `pnpm test:e2e:critical`. |
| `NVBES_SMOKE_WEB_MARKER`        | Non         | Marqueur HTML attendu par `pnpm test:smoke`, defaut `nvbes`. |
| `NVBES_DATABASE_URL`            | Oui         | `pnpm test:e2e:critical` hermetique et preflight Stripe; jamais pour provisionner staging. |
| `NVBES_STAGING_WEB_BASE_URL`    | Oui         | `pnpm test:smoke:staging`, `pnpm release:gate:staging`.                |
| `NVBES_STAGING_API_BASE_URL`    | Oui         | `pnpm test:smoke:staging`, `pnpm release:gate:staging`.                |
| `NVBES_STAGING_DATABASE_URL`    | Oui         | `pnpm db:migrate:staging`, `pnpm test:smoke:staging`, `pnpm release:gate:staging`. |
| `NVBES_ALLOW_STAGING_MIGRATION` | Oui         | Doit valoir `yes` pour migrer staging.                                 |
| `NVBES_BETA_SEED_EMAIL`         | Oui local   | Identite synthetique des tests hermetiques uniquement.                 |
| `NVBES_BETA_SEED_PASSWORD`      | Oui local   | Secret injecte par environnement, jamais passe en argument de processus. |
| `NVBES_BETA_SEED_WORKSPACE`     | Non local   | Nom workspace de la fixture hermetique.                                |

## Donnees de Test Beta

La commande historique `beta:seed:staging` a ete retiree. Elle combinait des
appels HTTP avec un acces direct a PostgreSQL/Redis staging et pouvait exposer
un secret dans les arguments de processus. Une identite staging doit etre
provisionnee par le control plane, avec audit, rotation, expiration et secret
stocke dans l'environnement GitHub protege.

Les tests hermetiques locaux gardent un helper strictement test-only. Il refuse
les hôtes non loopback, les environnements staging/production, les noms de base
sans segment `test` exact et toute execution sans opt-in destructif explicite.
Ne pas utiliser de vrais fichiers client pour la beta interne.

## Scripts de Migration

Preflight local:

```bash
pnpm format:check
pnpm lint
pnpm check
pnpm test:unit
pnpm test:integration
```

Migration staging:

```bash
NVBES_STAGING_DATABASE_URL="postgres://..." \
NVBES_ALLOW_STAGING_MIGRATION=yes \
pnpm db:migrate:staging
```

Regles:

- Confirmer backup/snapshot staging avant `NVBES_ALLOW_STAGING_MIGRATION=yes`.
- Appliquer les migrations avant de lancer le nouveau binaire API/worker.
- Verifier `/health` puis `/metrics`.
- Lancer `pnpm test:smoke:staging`.
- Si une migration echoue, stopper le rollout et ouvrir incident SEV2 tant que staging est inutilisable.

## Smoke Tests Staging

Commande:

```bash
NVBES_STAGING_WEB_BASE_URL="https://app.staging.example.com" \
NVBES_STAGING_API_BASE_URL="https://api.staging.example.com" \
pnpm test:smoke:staging
```

Checks couverts:

- web accessible et contient `nvbes Drive`;
- `/health` retourne `ok`;
- propagation `x-request-id`;
- `/metrics` expose le schema metriques;
- route publique partage montee sans 5xx.

Checks manuels beta a faire en plus:

- signup + verification email + login;
- `/v1/me` rejette les requetes sans credentials;
- dashboards, alertes critiques et log streams exposes;
- upload metadata + completion;
- lien public accessible puis revoque;
- audit visible par owner;
- cle API creee et rejetee si scope manquant;
- billing overview accessible;
- webhook Stripe test signe.

## Runbooks Incidents

Runbook principal: `docs/operations/incident-runbooks.md`.

Minimum avant beta:

- API down;
- PostgreSQL down/sature;
- Object Storage indisponible;
- bucket public suspect;
- billing webhook failure;
- jobs RGPD bloques;
- rollback applicatif apres migration.

## Go/No-Go Beta

Go uniquement si:

- `pnpm release:gate:staging` vert;
- `pnpm test:smoke:staging` vert apres deploiement;
- aucun P0/P1 ouvert sur auth, permissions, upload, partage public, audit, billing, privacy;
- backup staging recent confirme;
- owner incident et canal d'alerte beta designes;
- points bloquants ci-dessous traites ou acceptes.

## Points Bloquants Restants

### Bloquants avant beta externe

- Email transactionnel reel a valider en staging: verification, reset password et invitations doivent bien partir via le provider, avec token lisible uniquement via un helper interne non public.
- Stripe mappings staging a finaliser: `stripe_price_mappings` doit contenir de vrais IDs test/staging pour `solo_pro`, `team` et `team_plus`, avec preflight explicite avant checkout beta.
- E2E critiques a elargir si l'on veut couvrir plus que le noyau actuel: le parcours navigateur Playwright sur `account-web` couvre les flux critiques, mais les autres parcours beta restent a ajouter pour une couverture plus large.

### Branches deja reliees au flux de release

- OpenAPI Identity versionnee generee dans `apps/identity-service/openapi.json` puis republiee vers `libs/ts/identity-sdk-core/openapi.json` via `pnpm generate:openapi`. Le contrat Account distinct est genere dans `apps/account-service-next/openapi.json`.

### Risques acceptables pour beta interne fermee

- Object Storage peut rester simule tant que la beta reste fermee et que les URLs locales sont des URLs applicatives de forme `storage.<env>.nvbes.local`.
- Anti-malware/quarantaine reste hors code tant que l'exposition publique large n'est pas visee.
- Billing peut rester en mode Stripe test si les beta users ne payent pas.
- Donnees seed non representatives de gros volumes.
- UI encore shell/donnees locales pour certaines vues, si le protocole beta teste surtout les API/parcours techniques.

### Decisions a prendre

- Beta interne uniquement ou beta client pilote.
- Nombre maximum de workspaces beta.
- Retention des donnees beta et date de purge.
- Canal support et temps de reponse beta.
- Seuil d'incident qui suspend les invitations beta.
