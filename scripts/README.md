# Scripts

Scripts projet partages pour bootstrap local, checks et automatisations simples.

## Convention

- Prefixer par domaine quand un script devient specifique: `dev:*`, `check:*`, `lint:*`, `build:*`.
- Garder les scripts racine comme point d'entree principal du repo.
- Eviter la logique complexe inline dans `package.json`; la deplacer ici quand elle grossit.
- Ajouter un script dédié quand une règle de validation doit etre exécutable en local et en CI.

## Tests executables

- `test-unit.sh`: micro-tests Rust des manifestes V1 et tests TypeScript actifs,
  sans chargement de `.env` ni provisionnement. Le reseau est bloque pendant
  l'execution (Seatbelt macOS, seccomp Linux), y compris le loopback.
  Voir [Micro-tests et isolation](../docs/testing/micro-tests-isolation.md).
- `test-workspace-coverage.sh`: couverture `cargo llvm-cov` du workspace Cargo
  racine puis validation des seuils par crate (`docs/testing/rust-coverage-thresholds.json`).
- `pnpm test:rust:mutation`: mutation testing exhaustif des crates de production
  du catalogue V1, seuil minimum 90%, rapports complets obligatoires.
- `test-workspace-mutation.sh` (`pnpm test:rust:mutation:baseline`): diagnostic historique des crates
  gated puis validation des seuils par crate
  (`docs/testing/rust-mutation-thresholds.json`). Exige `cargo-mutants`
  (`cargo install cargo-mutants --locked --version 27.1.0`);
  `NVBES_MUTATION_TIMEOUT` regle le
  timeout mutant par defaut 600s et `NVBES_MUTATION_JOBS` permet de borner le
  parallelisme (par defaut, cargo-mutants choisit sa strategie d'execution).
- `test-workspace-condition.sh`: couverture de condition (`cargo llvm-cov
--branch`) du workspace Cargo puis validation des seuils par crate
  (`docs/testing/rust-condition-thresholds.json`). Exige un toolchain nightly
  (`rustup toolchain install nightly --profile minimal`); gate hors CI,
  `NVBES_CONDITION_TOOLCHAIN` regle le toolchain par defaut `nightly`.
- `generate-test-summary-report.mjs` (`pnpm report:test-summary`): génère le Test
  Summary Report release conforme au template ISO 29119-3 §6 visé par
  `docs/testing/iso-29119/03-documentation.md`, à partir du manifeste de test
  Account, du rapport `cargo llvm-cov` et des seuils par crate. Écrit dans
  `.temp/test-summary/test-summary-report.md`; `TSR_RELEASE` fixe la version notée.
- `test-integration.sh`: tests d'integration Rust et validation IaC development/staging.
- `test-account-portfolio.sh`: portefeuille bloquant Account complet (applications,
  bibliotheques TS associees, contrats, migrations, securite, worker, image et
  couverture HTTP) utilise par le release gate.
- `lint-account-rust.sh`: Clippy bloquant `all-targets`/`all-features` avec
  `-D warnings` pour Account Service, Account Worker et les crates Rust
  associees declarees dans le manifeste.
- `test-openapi-contract.sh`: smoke contractuel OpenAPI sur les routes publiques et proxifiées.
- `test-openapi-contract-seeded-auth.sh`: smoke contractuel OpenAPI avec seed/login auth pour couvrir aussi une partie des routes protegées/stateful.
- `test-e2e-critical.sh`: parcours critiques navigateur contre un environnement deploye.
- `test-identity-e2e-local.sh`: démarre un environnement Identity hermétique local/CI, exécute les parcours critiques puis détruit les ressources isolées.
- `test-smoke.sh`: checks rapides post-deploiement web/API.
- `smoke-staging.sh`: wrapper staging qui lance le smoke public puis une acceptation Account authentifiée avec une identité synthétique.
- `check-stripe-mappings.sh`: preflight staging pour valider les mappings Stripe actifs.
- `migrate-staging.sh`: applique les migrations PostgreSQL sur staging apres confirmation explicite.
- `release-gate.sh`: gate staging et gate production post-déploiement; ce dernier exige
  le paquet d'acceptation Account signé, l'attestation control-plane du release et le
  smoke sur les URLs de production.
- `check-llm-structure.sh`: verifie la platitude de `src/`, et les seuils de taille des fichiers Rust et TypeScript/frontend (< 500 lignes bloquant, alerte > 300 lignes).
- `dev.sh`: prépare les bases locales puis lance Email, Trust/Risk, Identity, Account et Billing.
- `dev-account-service.sh`: lance Account en isolation sur le port local 3070.
- `dev-billing-service.sh`: lance le workspace Cargo Billing autonome sur le port local 3080.
- `dev-email-worker.sh`: génère les templates et lance Email avec rechargement à chaud si `cargo-watch` est disponible.
- `dev-identity-service.sh`: lance Identity en isolation sur le port local 3060.
- `dev-trust-risk-service.sh`: lance Trust/Risk en isolation sur le port local 3050.
- `generate-openapi.sh`: regenere les specs OpenAPI Identity, Account, Developer, Cloud et Backoffice, puis republie les SDK generes.

Variables attendues pour les tests deployes:

- `NVBES_WEB_BASE_URL`
- `NVBES_API_BASE_URL`
- `NVBES_SMOKE_WEB_MARKER` (optionnel, defaut `nvbes`) pour verifier le HTML web attendu
- `NVBES_SMOKE_INCLUDE_SEEDED_AUTH=1` pour activer le smoke contract seeded-auth depuis `test-smoke.sh`
- `NVBES_SMOKE_AUTH_EMAIL` et `NVBES_SMOKE_AUTH_PASSWORD` (optionnels) pour reutiliser un compte existant au lieu d'en seeder un nouveau
- `NVBES_ALLOW_DESTRUCTIVE_TEST_DATABASE=account-quality-v1` pour toute lane
  Account autorisee a creer, migrer ou supprimer des ressources PostgreSQL;
  la base doit etre loopback, avoir un segment `test` exact et aucun segment
  production-like.
- `NVBES_IDENTITY_TEST_DATABASE_URL` pour les tests SQL Identity Service et
  Identity Worker; cette URL est validee par les memes garde-fous destructifs.

Variables attendues par les gates:

- `NVBES_STAGING_WEB_BASE_URL`
- `NVBES_STAGING_API_BASE_URL`
- `NVBES_STAGING_ACCOUNT_EMAIL` et `NVBES_STAGING_ACCOUNT_PASSWORD` pour l'acceptation navigateur authentifiée des release gates
- `NVBES_STAGING_ALLOWED_WEB_ORIGINS` et
  `NVBES_STAGING_ALLOWED_API_ORIGINS` pour l'allowlist exacte, HTTPS et
  resolue publiquement des cibles staging avant toute mutation
- `ACCOUNT_PRODUCTION_DENIED_ORIGINS` pour la denylist protegee des alias de
  production, appliquee au preflight Node et a nouveau dans le runtime k6
- `NVBES_STAGING_BILLING_DATABASE_URL` pour le preflight Stripe du release gate staging
- `NVBES_BILLING_DATABASE_URL` pour `check-stripe-mappings.sh` hors release gate
- `RELEASE_APPROVED=production` pour le gate production
- `NVBES_PRODUCTION_WEB_BASE_URL` et `NVBES_PRODUCTION_API_BASE_URL` pour le smoke production post-deploiement
- `NVBES_STAGING_DATABASE_URL` et `NVBES_ALLOW_STAGING_MIGRATION=yes` pour `db:migrate:staging`
- `NVBES_DATABASE_URL` pour `test-e2e-critical.sh`; ce point d'entree
  refuse staging/production et exige une base loopback jetable.
- `NVBES_BETA_SEED_EMAIL`, `NVBES_BETA_SEED_PASSWORD`,
  `NVBES_BETA_SEED_WORKSPACE` pour les fixtures hermetiques locales. Les
  identites staging sont provisionnees hors de ce repo par le control plane et
  fournies aux gates via des secrets proteges.
