# Strategie de Test

> **Statut : stratégie historique mixte.** Les principes de qualité restent
> applicables, mais les scénarios Cloud/Drive ou Enterprise ne sont pas des
> gates du socle V1. Le parcours intégré actif couvre Identity, Account,
> Billing, Email, Trust/Risk et Platform Operations.

## Objectif

Definir la strategie de qualite V1 pour nvbes Drive et aligner les tests sur les risques produit, techniques et business.

## Principes

- Tester au niveau le plus bas pertinent.
- Reserver les E2E aux parcours critiques et aux integrations reelles.
- Avoir une suite de regression stable avant lancement public.
- Separer clairement tests unitaires, integration, E2E, smoke, UI, accessibilite et UX research.
- Tout bug de production significatif doit conduire a un renforcement de la suite de regression.

## Pyramide cible

1. Tests unitaires.
2. Tests d'integration.
3. Tests E2E critiques.
4. Smoke tests de deploiement.
5. Verifications UI, accessibilite et UX ciblees.

## Categories

### Tests unitaires

But:

- Verifier la logique metier isolee.
- Capturer les cas limites et invariants.

Cibles prioritaires V1:

- permissions et roles;
- calculs de quotas;
- expiration et revocation de liens;
- validation upload;
- logique de billing usage snapshot;
- invalidation de sessions;
- sanitation/redaction des logs;
- workers purs et fonctions de planification.

Regles:

- Pas d'appel reseau reel.
- Pas de dependance provider reelle.
- Mocks/stubs limites aux frontieres externes.
- Chaque bug metier reproduit doit d'abord obtenir un test unitaire si le niveau est approprie.

### Tests d'integration

But:

- Verifier la collaboration entre modules, base, object storage, webhooks et jobs.

Cibles prioritaires V1:

- auth + session + verification email;
- upload session + object activation + quota;
- partage public + expiration + revocation;
- mutation role + invalidation sessions;
- webhook billing signe + idempotence + projection interne;
- export/suppression privacy;
- purge corbeille et uploads expires;
- audit event emission.

Regles:

- Utiliser une base et des services de test dedies ou emulations proches.
- Verifier les effets persistants, pas seulement les statuts HTTP.

### Tests E2E

But:

- Verifier les parcours utilisateur complets sur environnement proche reel.

### Standard frontend V1

Tout frontend produit réintroduit dans le périmètre V1 doit utiliser la même
chaîne de validation :

- **Vitest** pour les tests unitaires et de composants ;
- **Testing Library** (`@testing-library/react`, `@testing-library/user-event`
  et `@testing-library/jest-dom`) pour tester le comportement observable des
  composants React ;
- **Playwright** (`@playwright/test`) pour les parcours E2E navigateur ;
- **axe-core** via `@axe-core/playwright` pour les contrôles
  d’accessibilité automatisés sur les parcours critiques.

Règles de wiring :

- chaque package frontend doit exposer des cibles Nx `test` et
  `test:e2e` adaptées à son périmètre ;
- les tests Vitest doivent rester rapides, déterministes et sans navigateur
  réel ;
- Playwright doit couvrir uniquement les parcours critiques et s’exécuter
  contre un environnement V1 explicitement ciblé ;
- chaque parcours Playwright critique doit inclure un contrôle axe-core
  ciblé, sans remplacer les tests unitaires ou les vérifications manuelles ;
- les versions et scripts doivent être déclarés dans le package frontend
  réintroduit, puis validés avec `pnpm install --frozen-lockfile`.

Les frontends archivés ne constituent pas une source d’implémentation :
`account-web`, `identity-web`, `backoffice-web`, `cloud-web`, `console-web`
et les autres applications archivées devront appliquer ce standard lors d’une
réintroduction conforme à la direction V1.

Parcours minimum V1:

- signup -> creation workspace -> premier upload;
- creation lien partage -> acces public -> revocation;
- invitation membre -> acceptance -> acces limite;
- quota presque atteint -> alerte -> blocage upload si depassement;
- checkout/upgrade en environnement de test billing;
- creation/revocation cle API.

Regles:

- Nombre volontairement limite.
- Stables, rapides a diagnostiquer, alignes sur les parcours coeur.
- Executables en staging avant promotion.

### Smoke tests

But:

- Confirmer que l'environnement deploye est sain.

Checks minimum:

- page web accessible;
- login disponible;
- API healthy;
- upload simple;
- route partage public repond;
- database connectee;
- workers actifs;
- webhooks critiques en file d'attente sains.

### Tests de regression

But:

- Eviter la reintroduction de bugs connus et proteger les parcours critiques avant release.

Composition V1:

- sous-ensemble stable des tests unitaires critiques;
- integrations critiques;
- E2E coeur;
- checks UI sur etats critiques;
- checks accessibilite automatises minimaux.

Declencheurs:

- avant release staging;
- avant promotion production;
- apres correction de bug critique;
- apres changement sur auth, permissions, upload, billing, privacy ou partage public.

## Gates de release

Scripts executables:

- `pnpm test:unit`: typecheck web et tests unitaires Rust.
- `pnpm test:rust:coverage`: couverture `cargo llvm-cov` sur tout le workspace Rust avec seuils par crate (`docs/testing/rust-coverage-thresholds.json`).
- `pnpm test:rust:mutation`: mutation testing `cargo-mutants` sur les crates Rust incluses avec seuils versionnés (`docs/testing/rust-mutation-thresholds.json`).
- `pnpm test:rust:condition`: branches instrumentées via `cargo llvm-cov --branch` (nightly épinglé), diagnostic historique hors CI ; ne prouve ni MC/DC ni les seuils V1.
- `pnpm test:integration`: tests d'integration Rust et validation IaC development/staging.
- `pnpm test:e2e:critical`: E2E critiques contre `NVBES_WEB_BASE_URL` et `NVBES_API_BASE_URL`.
- `pnpm test:smoke`: smoke tests contre `NVBES_WEB_BASE_URL` et `NVBES_API_BASE_URL`.
- `pnpm test:smoke:staging`: wrapper staging pour smoke + E2E critiques avec les URLs staging.
- `pnpm release:gate:staging`: décision V1 fondée sur le paquet de preuves vérifié, identique au Test Summary V1 ; échoue si une suite obligatoire manque. Ne déploie pas et ne remplace pas les campagnes.
- `pnpm release:gate:production`: gate post-déploiement avec approval explicite,
  preuves d'acceptation signées et smoke production obligatoire.

Avant staging:

- lint/format;
- tests unitaires;
- tests d'integration rapides;
- checks UI composant critiques si modifies.
- `pnpm release:gate:staging` avec URLs staging quand l'environnement est deploye.

Avant production:

- smoke staging verts;
- suite de regression critique verte;
- migrations verifiees;
- approval explicite.
- `RELEASE_APPROVED=production pnpm release:gate:production`.

Notes d'implementation:

- La suite `pnpm test:e2e:critical` est branchee sur les parcours navigateur critiques d'`identity-web` via Playwright.
- `pnpm test:smoke` couvre les checks HTTP rapides actuellement implementes; les checks applicatifs listes plus haut restent a brancher si requis pour la gate.
- L'extension de couverture E2E reste un sujet de contenu de test, pas un blocage de wiring release.

Apres production:

- smoke production verts;
- monitoring sans alerte critique;
- plan de rollback connu.

## Ownership

- Engineering: unit, integration, E2E, smoke.
- Product design: UI states, UX scenarios, test d'utilisabilite.
- QA ou owner technique: orchestration regression pre-release.
- Security/privacy owner: scenarios auth, audit, RGPD, consentement.
