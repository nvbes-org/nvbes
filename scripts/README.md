# Scripts

Scripts projet partages pour bootstrap local, checks et automatisations simples.

## Convention

- Prefixer par domaine quand un script devient specifique: `dev:*`, `check:*`, `lint:*`, `build:*`.
- Garder les scripts racine comme point d'entree principal du repo.
- Eviter la logique complexe inline dans `package.json`; la deplacer ici quand elle grossit.
- Ajouter un script dédié quand une règle de validation doit etre exécutable en local et en CI.

## Tests executables

- `test-unit.sh`: typecheck web et tests unitaires Rust.
- `test-integration.sh`: tests d'integration Rust et validation IaC development/staging.
- `test-openapi-contract.sh`: smoke contractuel OpenAPI sur les routes publiques et proxifiées.
- `test-openapi-contract-seeded-auth.sh`: smoke contractuel OpenAPI avec seed/login auth pour couvrir aussi une partie des routes protegées/stateful.
- `test-e2e-critical.sh`: parcours critiques navigateur contre un environnement deploye.
- `test-smoke.sh`: checks rapides post-deploiement web/API.
- `smoke-staging.sh`: wrapper staging qui lance smoke + E2E critiques avec les URLs staging.
- `seed-beta-data.sh`: cree un workspace beta, un dossier, un objet, un lien public et une cle API sur un environnement non-production.
- `check-stripe-mappings.sh`: preflight staging pour valider les mappings Stripe actifs.
- `migrate-staging.sh`: applique les migrations PostgreSQL sur staging apres confirmation explicite.
- `release-gate.sh`: gates staging et production, avec build, preflight Stripe et E2E critiques pour staging.
- `check-llm-structure.sh`: verifie la platitude de `src/`, et les seuils de taille des fichiers Rust.
- `dev-identity-worker.sh`: lance le worker Identity en isolation.
- `dev-drive-worker.sh`: lance le worker Drive en isolation.
- `dev-drive-db-reset.sh`: recree la base Drive locale `nvbes_drive` quand les checksums SQLx dev ne correspondent plus.
- `dev-worker.sh`: alias historique vers le worker Identity.
- `generate-openapi.sh`: regenere les specs OpenAPI et republie `libs/ts/identity-sdk-core/openapi.json`.

Variables attendues pour les tests deployes:

- `NVBES_WEB_BASE_URL`
- `NVBES_API_BASE_URL`
- `NVBES_SMOKE_WEB_MARKER` (optionnel, defaut `nvbes`) pour verifier le HTML web attendu
- `NVBES_SMOKE_INCLUDE_SEEDED_AUTH=1` pour activer le smoke contract seeded-auth depuis `test-smoke.sh`
- `NVBES_SMOKE_AUTH_EMAIL` et `NVBES_SMOKE_AUTH_PASSWORD` (optionnels) pour reutiliser un compte existant au lieu d'en seeder un nouveau
- `NVBES_SMOKE_AUTH_TURNSTILE_TOKEN` (optionnel) si l'endpoint `/auth/challenge/identifier` impose Turnstile

Variables attendues par les gates:

- `NVBES_STAGING_WEB_BASE_URL`
- `NVBES_STAGING_API_BASE_URL`
- `NVBES_STAGING_DATABASE_URL` pour `smoke-staging.sh`, `release-gate.sh`, les E2E critiques et le preflight Stripe staging
- `RELEASE_APPROVED=production` pour le gate production
- `NVBES_PRODUCTION_WEB_BASE_URL` et `NVBES_PRODUCTION_API_BASE_URL` pour le smoke production post-deploiement
- `NVBES_STAGING_DATABASE_URL` et `NVBES_ALLOW_STAGING_MIGRATION=yes` pour `db:migrate:staging`
- `NVBES_DATABASE_URL` pour `test-e2e-critical.sh`, `beta:seed:staging` et `check-stripe-mappings.sh`
- `NVBES_IDENTITY_API_BASE_URL` et `NVBES_DRIVE_API_BASE_URL` pour `beta:seed:staging` quand Identity et Drive ne sont pas agreges derriere `NVBES_API_BASE_URL`
- `NVBES_BETA_SEED_EMAIL`, `NVBES_BETA_SEED_PASSWORD`, `NVBES_BETA_SEED_WORKSPACE` pour `beta:seed:staging`
