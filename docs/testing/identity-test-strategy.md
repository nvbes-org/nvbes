# Stratégie de test Identity

> **Statut : socle V1 actif.** L'identity-service est un runtime fermé : il n'expose
> aucune route HTTP publique d'auth. Auth, sessions, MFA et tokens sont des fonctions
> internes prouvées par les commandes `synthetic-*-smoke` et par les tests unitaires,
> conformément à la [direction produit](../product/nvbes-product-strategy.md).

## Portée et statut de preuve

Cette stratégie couvre `apps/identity-service` et les crates directes
(`nvbes-core` auth/MFA, `nvbes-audit`, `nvbes-observability`, `nvbes-identity-sdk`,
`nvbes-region`, `nvbes-tenancy`). Le socle V1 ne déclare pas encore de
manifeste machine-readable par domaine pour Identity : chaque catégorie est
renseignée ici avec son lane et sa preuve réelle, et une catégorie sans harness
est marquée explicitement plutôt que présumée couverte.

L'interface externe réelle est volontairement réduite :

| Route HTTP          | Rôle                        |
| ------------------- | --------------------------- |
| `GET /health/live`  | liveness sans dépendance    |
| `GET /health/ready` | readiness (DB + migrations) |
| `GET /metrics`      | Prometheus (Bearer token)   |

Le test de contrat conteneur (`tests/container-contract.test.mjs`) **exige l'absence**
des routes publiques d'auth (`assert.equal(mainSource.includes('route("/auth/register"'), false)`).
Identité, sessions, TOTP, JWT et recovery passent par des fonctions internes et par
les commandes CLI `synthetic-*` avant démarrage.

## État d'automatisation réel

| Lane             | Déclenchement réel                       | Couverture actuelle                                                                                                                                                                 | Preuve                |
| ---------------- | ---------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------- |
| PR               | CI du dépôt (scope planifié)             | `cargo fmt --all --check` + `cargo test --locked` : auth, config, database, email, health, metrics, mfa.crypto, tokens                                                              | rapports Cargo        |
| PR               | `cargo check --workspace`                | compilation workspace complète                                                                                                                                                      | Cargo                 |
| Database         | lane `database` CI avec PostgreSQL isolé | `identity.database.tests.rs`, integration avec migrations                                                                                                                           | `test:database` Nx    |
| Containers       | lane `containers` CI                     | contrat conteneur : reproducibility, non-root, migrations avant serveur, preuve smoke synthétique                                                                                   | `test:contract` Nx    |
| Trusted hermetic | commande opérateur                       | PostgreSQL éphémère migré depuis zéro, Identity + Redis isolés, puis commandes `synthetic-auth-smoke`, `synthetic-auth-email-smoke`, `synthetic-mfa-smoke`, `synthetic-token-smoke` | journaux smoke signés |
| Pre-release      | session signée                           | tests MFA/TOTP à rejouer en environnement signé, tests de réception et recovery                                                                                                     | rapports signés       |

Les commandes `synthetic-*` sont la preuve de bout en bout d'une identité dans le
runtime : création et authentification, vérification via email, TOTP (enrôlement,
confirmation, step-up, **rejet de rejeu**), JWT (émission, vérification, introspection,
révocation, audience). Un échec d'une commande smoke fait échouer le job, jamais un skip.

### Limites explicites

- Aucune route HTTP publique d'auth n'existe ni ne doit exister : le test de contrat
  conteneur l'assure. Un futur frontend s'appuie sur l'identity-sdk-backend et les
  fonctions internes, pas sur des endpoints API REST d'auth.
- Les tests `identity.mfa.crypto` couvrent le scellement AES-256-GCM et la rotation de
  clé; des menaces physiques (extraction de clé, timing) restent hors périmètre V1.
- MFA/TOTP n'a pas encore de campagne Playwright : le socle V1 est CLI/API, pas UI.
- La recovery (challenge 15 min, token) est testée en unitaire et via
  `synthetic-auth-smoke`; elle n'a pas de matrice de volume.
- `database-tests` est feature-gated et ne tourne que dans la lane `database`;
  un cargo test nu ne prouve pas l'intégration DB.

## Fiabilité et sécurité du harness

- Les commandes `synthetic-*` n'acceptent que des cibles de test validées
  (`NVBES_ENV` de type test/development/local), jamais staging/production
  (guard `env.validate_test`).
- JWT RS256 : les clés de signature sont injectées par configuration; le test
  `identity.tokens` vérifie émission/vérification/introspection contre la table de
  sessions et l'audience.
- Argon2 via `nvbes-core::auth` : aucun mot de passe en clair dans les journaux ni
  artefacts; les erreurs, logs et artefacts sont rédigés.
- Les tokens MFA sont des empreintes hachées; le rejeu TOTP est explicitement rejeté
  par le run smoke.

## Environnement et données de test

- PostgreSQL éphémère migré depuis zéro (3 fichiers de migration) pour la lane
  database et le runner hermétique.
- Redis requis pour les lanes hermétiques quand le runtime l'exige.
- Les données de test sont synthétiques; aucune donnée personnelle réelle.
- Budget : un seul conteneur serverless Identity, coût couvert par le plafond global
  de 30 EUR TTC/mois (FinOps gate).

## Suites ISO 29119-3

Suites du §3.2 de `03-documentation.md` et leurs commandes réelles :

| Suite ID           | Catégorie | Commande réelle                                               |
| ------------------ | --------- | ------------------------------------------------------------- |
| TS-IDENTITY-AUTH   | auth      | `cargo test --package nvbes-identity-service identity.auth`   |
| TS-IDENTITY-MFA    | security  | `cargo test --package nvbes-identity-service identity.mfa`    |
| TS-IDENTITY-TOKENS | tokens    | `cargo test --package nvbes-identity-service identity.tokens` |
| TS-IDENTITY-HEALTH | health    | `cargo test --package nvbes-identity-service identity.health` |

Les test cases suivent le template Test Case (§4 de `03-documentation.md`) avec
ID `TC-IDENTITY-<SUITE>-<NUM>`. Le runner keyword-driven (`nvbes-runner`) exécute les
cases YAML en infra (`infra.setup_db`, `infra.migrate`, `infra.health_check`,
`infra.cleanup`) et vérifie les preuves smoke via les commandes opérateur.

## Gates et acceptation

- PR : lint, check, unitaires, contrats conteneur, migrations.
- Avant staging : smoke synthétiques verts sur l'environnement déployé.
- Avant production : rejouer `synthetic-auth-smoke` + `synthetic-mfa-smoke` +
  `synthetic-token-smoke` contre staging, contrats conteneur verts, approval
  explicite signée.
- Un bug critique d'identité (auth, MFA, token) commence par un test unitaire rouge
  au niveau le plus bas qui le reproduit.

## Principes de qualité

- Pas de route publique d'auth ajoutée sans mise à jour du contrat conteneur.
- Pas de skip d'infrastructure, de schéma ou de provider devenu vert.
- Aucun retry ne transforme un échec en succès.
- Les tests DB refusent les cibles non-loopback/non-test.
- Aucune donnée personnelle, clé ou token dans les artefacts.
- La rotation de clés MFA doit rester couverte par un test unitaire dédié.
