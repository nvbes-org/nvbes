# nvbes Project

Monorepo full-stack avec Rust (Axum), TypeScript/React, orchestré par pnpm workspaces + Cargo workspace.

## Direction V1 active

La [direction produit canonique](docs/product/nvbes-product-strategy.md) prévaut
sur les anciens PRD, blueprints et plans :

- la V1 livre uniquement le socle réutilisable Identity, Account, Billing,
  Email, Trust/Risk et Platform Operations ;
- la cible est B2C tout public, avec équipes, sans offre B2B/Enterprise ni
  conformité avancée dans la V1 ;
- Cloud/Drive n'est pas le premier produit implicite et reste hors périmètre ;
- le coût récurrent total du projet ne doit pas dépasser 30 EUR TTC par mois ;
- FinOps est une propriété de conception et un gate de livraison ;
- un opérateur solo traite les demandes manuellement tant qu'une automatisation
  sûre, conforme aux exigences et compatible avec le budget n'est pas démontrée ;
- une solution maison optimisée est recevable si elle est moins coûteuse, sûre,
  maintenable et remplaçable ;
- la production est progressive, mesurée et réversible.

## Agent contract

- Lire ce fichier avant de modifier le codebase.
- Préférer les fichiers courts, plats, et auto-documentants.
- Ne pas introduire de dette technique provisoire sur les zones touchées.
- Ne pas renommer ou déplacer des fichiers hors scope sans raison de design claire.
- Utiliser Nx pour explorer le graphe de projets, les cibles et le périmètre affecté quand cela aide à réduire le contexte.
- Utiliser ripgrep pour toute recherche locale : `rg` pour rechercher du contenu et
  `rg --files` pour découvrir des fichiers. N'utiliser `grep` ou `find` qu'en
  solution de repli si `rg` est indisponible ou inadapté.
- Tout commit LLM doit porter le trailer `AI-Assisted: <agent-or-model>`.
- Le workflow est **100% LLM** : un agent peut modifier n'importe quel fichier,
  y compris les gates (`lefthook.yml`, `tools/ci/**`, `tools/security/**`,
  `docs/testing/*thresholds*`, `deny.toml`, `.github/**`, `commitlint.config.cjs`,
  `AGENTS.md`). Aucun gate de reprise humaine bloquant.
- Ne jamais abaisser un seuil de couverture / mutation / condition ni exclure un crate
  du gate (`pnpm check:thresholds-monotone`) — hard-block anti-Goodhart.
- Préférer `matches!` / `assert_eq!` aux `assert!(….is_err())` / `assert!(….is_ok())`
  dans les diffs nouveaux (`pnpm check:weak-assertions`).
- Pas de nouveau `.unwrap()` / `.expect()` hors tests (`pnpm check:unwrap-ratchet`).
- Pas de nouveau `sqlx::query(` runtime : viser `query!` (`pnpm check:sqlx-ratchet`).

## Stack

- **Package manager**: pnpm
- **Formatter/Linter**: Biome
- **Hooks git**: Lefthook
- **Rust**: stable (Cargo workspace)
- **Frontend**: React, Vite, TanStack Router, TanStack Query, Effect
- **Backend**: Axum, SQLx, PostgreSQL
- **Auth**: nvbes Identity (web), JWT (API)
- **Billing**: Stripe

## Structure

```
nvbes/
├── apps/
│   ├── email-worker/       # Runtime Email actif
│   ├── identity-web/       # UI hébergée Identity (OAuth/MFA)
│   ├── account-web/        # UI Account (session, logout RP)
│   └── trust-risk-service/ # Runtime Trust/Risk actif
├── libs/
│   ├── rust/
│   │   ├── core/           # Primitives partagées (config, auth, mfa)
│   │   ├── adapters/       # Adaptateurs externes actifs (Scaleway email)
│   │   ├── audit/          # Audit append-only
│   │   ├── billing/        # Logique billing partagée
│   │   ├── email/          # Service email
│   │   ├── observability/  # Métriques, tracing, sentry
│   │   ├── platform/       # Primitives de plateforme
│   │   ├── region/         # Régions et résidence des données
│   │   ├── redis/          # Primitives Redis actives
│   │   └── trust-risk/     # Domaine Trust/Risk
│   └── ts/
│       ├── http-client/        # Transport fetch partagé (DTO + DPoP helpers)
│       ├── identity-sdk-core/  # Types OpenAPI générés
│       ├── identity-sdk-web/   # SDK navigateur Identity (OAuth/DPoP/WebAuthn)
│       └── email-ui/           # Templates React Email actifs
├── infrastructure/         # Terraform/OpenTofu et contrat FinOps
└── archive/                # Produits et prototypes hors runtime actif
```

Les applications archivées Account, Cloud, Developer, Enterprise et Backoffice
ne sont pas des bases d'implémentation actives. Leur réintroduction exige une
conception conforme à la direction V1 ; ne pas copier le code archivé par défaut.

## Repository instructions for agents

- `AGENTS.md` contient les règles générales du repo pour les agents de développement.
- `.codex/instructions.md` contient les instructions consolidées pour Codex.
- Les règles les plus proches du code prennent priorité sur les règles globales quand il y a conflit.

## Conventions de nommage (dot-notation)

Les fichiers Rust utilisent une convention plate avec des noms qualifiés par des points, pour une navigation LLM optimale :

```
apps/account-service/src/
├── identity.app.rs                    # AppState
├── identity.domains.billing.mod.rs    # module billing
├── identity.domains.billing.service.rs# logique métier
├── identity.domains.billing.routes.rs # routes HTTP
├── identity.http.error.rs             # erreurs HTTP
└── identity.http.middleware.jwt.rs    # middleware JWT
```

Les modules Rust sont déclarés avec `#[path]` :

```rust
// identity.domains.billing.mod.rs
#[path = "identity.domains.billing.service.rs"]
pub mod service;
#[path = "identity.domains.billing.routes.rs"]
pub mod routes;
```

**Règles** :

- Tous les fichiers `.rs` sont dans `src/` (pas de sous-répertoires profonds)
- Le préfixe (`identity.`, `drive.`, `backoffice.`) identifie la crate ou le domaine historique conservé dans le fichier
- Le suffixe (`service`, `routes`, `db`) identifie la couche
- Les modules Rust restent organisés logiquement via `mod.rs`

## Limites de taille de fichier

| Seuil      | Action                    |
| ---------- | ------------------------- |
| 300 lignes | Envisager l'extraction    |
| 500 lignes | **Obligation** d'extraire |

Fichiers sous 300 lignes = contexte LLM optimal. Le LLM peut lire un fichier entièrement sans diluer son attention.

## Philosophie d'architecture

- Favoriser le graphe de projets et les frontières de domaine nettes plutôt qu’un gros dossier monolithique.
- Préserver l’indépendance des produits Account, Cloud, Billing, Developer, Enterprise et Backoffice.
- Partager seulement les primitives réellement transverses.
- Toute extraction doit réduire la taille cognitive du code, pas seulement déplacer des lignes.
- Le backlog de refactor agentique est documenté dans [docs/blueprint/nvbes-monorepo-agentic-refactor.work.md](docs/blueprint/nvbes-monorepo-agentic-refactor.work.md).

## Commandes

```bash
pnpm dev              # Runtimes locaux principaux
pnpm dev:identity-service
pnpm dev:account-service
pnpm dev:billing-service
pnpm dev:email-worker
pnpm dev:trust-risk-service
pnpm check            # Checks complets
pnpm lint             # Lint complet
pnpm test             # Tests complets
pnpm verify           # Vérification pre-commit

# Web/TS
pnpm lint:web
pnpm check:web

# Rust
cargo check --workspace

# Nx
pnpm nx:show-projects
pnpm nx:show-project <project>
pnpm nx:affected
```

## Validation attendue

- Après changement Rust, exécuter `cargo check --workspace`.
- Après changement frontend, exécuter le check ciblé du package touché.
- Après changement transversal, exécuter les checks racine pertinents et documenter les éventuelles limites.

## Règles de développement

### Contexte V1 finale

- Le travail actif vise le **socle V1**, pas un produit final.
- Aucune dette technique ou structurelle connue n'est acceptable dans le
  périmètre livré.
- Si une partie du design ou de l'implémentation est bancale, le **redesign
  complet** est hautement préféré et encouragé.
- Éviter les rustines, les compromis temporaires et les extensions
  incrémentales sur une base faible.
- Après V1, toute dette acceptée doit avoir un propriétaire, une cause, un
  impact mesuré, une date de revue et une condition de résolution.
- Toute ressource ou automatisation nouvelle doit démontrer sa compatibilité
  avec le plafond global de 30 EUR TTC par mois.

# Hiérarchie DE SÉLECTION DE COMPOSANTS (OBLIGATOIRE — TOUS LLM)

**Pour TOUS les sites web/projets frontend, suivre cet ordre STRICTEMENT, sans exception :**

1. **Registry du projet** — Utiliser d'abord les composants du registry interne du projet (`apps/*/components/ui`, registry local `components.json`)
2. **Registry officiel shadcn/ui** — Si aucun composant utile dans le registry projet, utiliser le registry officiel shadcn/ui
   - Docs: https://ui.shadcn.com/docs/registry/getting-started
   - Directory: https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/directory.json
   - Installation: `pnpm dlx shadcn@latest add <component>`
3. **Registry externe compatible** — Chercher un registry tiers avec des composants utilisables (ex: `@radix-ui/themes`, `@headlessui/react`, registries communautaires)
4. **Tailwind CSS** — Si aucun composant ne correspond, écrire le composant avec Tailwind CSS (classes utilitaires uniquement)
5. **CSS pur / CSS Modules** — UNIQUEMENT en dernier recours, si Tailwind n'est pas disponible ou incompatible

**RÈGLES IMPÉRATIVES :**

- **NE JAMAIS** faire de solution "fait maison" (custom from scratch) avant d'avoir épuisé les 4 premiers niveaux
- Tout composant custom DOIT être justifié par écrit (pourquoi rien des niveaux 1-4 ne convient)
- Pour **TOUT NOUVEAU site web** : installer **impérativement** dans cet ordre → `tailwindcss` → `shadcn/ui` → registries nécessaires
- Pas d'exception, pas de "vite fait" — la hiérarchie est non-négociable

### Styling et design frontend

- Éviter de nester des cards dans des cards. Une card doit représenter un bloc autonome; pour structurer l'intérieur d'une card, préférer des sections, listes, bordures, séparateurs, fonds subtils ou layouts non cardés.
- Éviter les emojis dans l'interface. Les icônes de composants ou de bibliothèques (ex: Lucide) restent autorisées; l'emoji cookie est toléré uniquement pour les éléments liés aux cookies.

### Conception LLM-friendly

Le codebase est conçu pour être navigable par des LLMs (Claude Code, Cursor) :

- **Fichiers < 300 lignes** : lecture complète en un appel, pas de dilution d'attention
- **Noms auto-documentants** : le nom du fichier dit exactement ce qu'il contient
- **Structure plate** : pas de nesting profond, le LLM trouve les fichiers en 1 Glob
- **Modules explicites** : `mod.rs` avec `#[path]` fait le lien entre le fichier plat et le module Rust
- **Pas de `utils.rs` / `helpers.rs`** : chaque concept a son fichier dédié
- **Pas de logique cachée dans les barrels** : les `mod.rs` déclarent les modules, ils ne portent pas la logique métier
- **Frontiers de domaine explicites** : si deux responsabilités diffèrent, elles doivent vivre dans des fichiers séparés

### Patterns de code

- Pas d'imports inutiles ou de ré-exports redondants
- Utiliser les types et traits corrects (ex: `&PgPool` directement, pas de wrapper inutile)
- **Dépendances explicites** : Préférer passer les dépendances exactes (`&PgPool`, `&Config`) aux fonctions métier plutôt que l'objet global `&AppState` (évite le pattern Service Locator).
- Respecter les conventions Rust : `thiserror` pour les erreurs, pas de `anyhow` dans les crates libs
- SQLx : utiliser `&state.db` (PgPool) directement, pas de méthode `.pool()`
- TypeScript : INTERDICTION STRICTE du type `any`. Utiliser `unknown` ou des types précis. Pas de passe-droit.

### Compilation

- **TOUJOURS** faire `cargo check --workspace` après chaque modification Rust
- Corriger les erreurs de compilation immédiatement, pas de commits avec warnings/errors
- Pas de `#[allow(unused)]` ou de suppressions de warnings sans justification

### Règles de conception

- Identity, Account, Billing, Email, Trust/Risk et Platform Operations gardent
  des sources de vérité et des contrats distincts.
- Réécrire le code si nécessaire plutôt que de copier le code produit archivé.
- Les capacités livrées doivent être complètes ou absentes, jamais à moitié
  implémentées.
- Le socle ne dépend pas de Cloud, Drive ni d'un autre produit final.
- Mutualiser une capacité ne signifie pas créer automatiquement un microservice.
- Le traitement opérateur reste manuel par défaut ; automatiser uniquement un
  besoin mesuré, critique ou protecteur du budget.

### Architecture des Services

- **Services spécialisés** : Éviter les "God Objects" (un seul gros service par domaine). Préférer scinder en capacités métier (ex: `SubscriptionService`, `CheckoutService`).
- **Module-as-a-Service** : Si un service n'a pas d'état interne, privilégier des fonctions simples dans un module plutôt qu'une struct avec des méthodes.
- **Séparation des couches** : Maintenir une séparation nette entre persistence (`db.rs`), validation métier pure (`validation.rs`) et orchestration (`service.rs` ou `logic.rs`).

## Règles de Qualité et Sécurité en Amont (SonarQube, CodeQL, Security CI, CI)

Pour éviter les allers-retours avec les pipelines CI distants et SonarQube, tout agent et développeur doit appliquer STRICTEMENT ces règles en amont dès la conception et l'écriture du code :

### 1. SonarQube / SonarCloud (Strict Quality Gate)
- **SQL (`plsql:DeleteOrUpdateWithoutWhereCheck`)** : Tout `UPDATE` ou `DELETE` SQL DOIT impérativement comporter une clause `WHERE` explicite. Ne jamais écrire d'`UPDATE` ou `DELETE` sans clause `WHERE`.
- **TypeScript / JavaScript (`typescript:S2871`)** : Tout appel `.sort()` sur un tableau DOIT impérativement fournir une fonction de comparaison explicite (ex: `(a, b) => a.localeCompare(b)` pour les chaînes, `(a, b) => a - b` pour les nombres). Le tri sans comparateur est interdit.
- **Gestion des rejets de Promise (`typescript:S6671`)** : Dans les listeners d'événements, workers et callbacks, ne rejeter une Promise qu'avec une instance d'`Error` (`reject(new Error(...))`), jamais avec un événement brut, un objet générique ou une chaîne.
- **Anti-ReDoS / Backtracking (`typescript:S8786`)** : Interdiction absolue des quantificateurs imbriqués ou regex à backtracking catastrophique (ex: remplacer `/=+$/` par `/={1,2}$/` ou des méthodes de chaîne `.endsWith()`, `.startsWith()`, `.slice()`).
- **Nombres pseudo-aléatoires sécurisés (`typescript:S2245`)** : Interdiction totale de `Math.random()` dans tout contexte de sécurité, authentification, sessions, tokens, génération de secrets, identifiants, decoys ou signaux bot. Utiliser exclusivement `crypto.getRandomValues(new Uint8Array(...))` ou un CSPRNG.
- **Parsing d'entiers (`typescript:S7773`)** : Toujours utiliser `Number.parseInt(...)` et `Number.parseFloat(...)` au lieu des fonctions globales non qualifiées `parseInt(...)` et `parseFloat(...)`.
- **Paramètres par défaut (`typescript:S7737`)** : Ne pas initialiser les paramètres par défaut de fonctions avec des expressions complexes ou des objets mutables recalculés ; déclarer des constantes au niveau module.
- **Rust et Asynchronicité (`rust:S7487`)** : Interdiction formelle d'appels bloquants dans un contexte asynchrone (ex: `child.wait()` ou `thread::sleep` dans Tokio). Utiliser `tokio::task::spawn_blocking` ou les équivalents asynchrones `tokio::process` / `tokio::time::sleep`.
- **Couverture de code sur le nouveau code (`new_coverage`)** : SonarCloud exige au minimum **80% de couverture** sur tout nouveau code. Tout code métier ou composant ajouté doit avoir ses tests unitaires/intégration associés. Les rapports LCOV TypeScript doivent pointer vers des chemins relatifs à la racine (`SF:apps/...` ou `SF:libs/...`).
- **Duplication** : Moins de 3% de code dupliqué sur le nouveau code.

### 2. CodeQL (Actions, JavaScript/TypeScript, Rust)
- **Injection de commandes** : Ne jamais interpoler de variables dynamiques dans des chaînes de commande shell (`exec`). Préférer `execFile` ou `spawn` avec des arguments en tableau typé.
- **Path Traversal** : Toujours valider et confiner les chemins résolus à l'aide de `path.resolve` et s'assurer qu'aucun segment `..` ne permet d'échapper au périmètre autorisé.
- **Sécurité des Workflows GitHub Actions** : Ne JAMAIS interpoler `${{ github.event... }}` directement dans des blocs `run:`. Toujours mapper les données GitHub dans des variables d'environnement `env:`.
- **Sécurité Rust** : Tout bloc `unsafe` doit être obligatoirement accompagné d'un commentaire `// SAFETY:` détaillant ses invariants et garanties de sécurité mémoire.

### 3. Security CI (OSV, Cargo Deny, Trivy, Gitleaks, Zizmor)
- **OSV Scanner (`osv-lockfiles`)** : Aucune dépendance comportant une CVE / GHSA active n'est tolérée dans `pnpm-lock.yaml` ou `Cargo.lock`. Résoudre immédiatement via des overrides dans `pnpm-workspace.yaml` ou des montées de version.
- **Cargo Deny** : Respect strict des licences autorisées (AGPL-3.0, MIT, Apache-2.0, BSD-3-Clause). Aucune advisory RUSTSEC tolérée. Pas de versions multiples non autorisées de crates.
- **Gitleaks** : Zéro secret, token, mot de passe ou clé privée commité dans le dépôt ou présent dans l'historique git.
- **Trivy** : Zéro vulnérabilité ou misconfiguration de sévérité HIGH ou CRITICAL dans les fichiers et conteneurs du projet.
- **Zizmor** : GitHub Actions strictement durcies (actions épinglées par SHA complet, permissions minimales `permissions: contents: read`).

### 4. CI & Gates NVBES
- **Seuils monotones (`pnpm check:thresholds-monotone`)** : Interdiction absolue de baisser un seuil de couverture, de mutation ou de condition dans `docs/testing/` ou d'exclure un crate.
- **Unwrap Ratchet (`pnpm check:unwrap-ratchet`)** : Zéro nouveau `.unwrap()` ou `.expect()` en code de production Rust.
- **SQLx Ratchet (`pnpm check:sqlx-ratchet`)** : Zéro nouveau `sqlx::query(` runtime ; utiliser `sqlx::query!` avec métadonnées `.sqlx` hors-ligne synchronisées.
- **Weak Assertions (`pnpm check:weak-assertions`)** : Utiliser `matches!` ou `assert_eq!` au lieu de `assert!(x.is_ok())` / `assert!(x.is_err())`.
- **Règles en amont automatisées (`pnpm check:upstream-rules`)** : Validation automatique locale des anti-patterns SonarQube et CodeQL avant commit.
- **Migrations DB** : Tout fichier de migration SQL doit comporter un `-- migrate:up` et un `-- migrate:down` symétriques et se terminer par un saut de ligne (`\n`).
- **Commits** : Conventional Commits avec lignes de corps inférieures à 80 caractères et trailer obligatoire `AI-Assisted: <nom-de-l-agent>`.

## Validation avant fin de tâche

- Après modification de code: lancer au minimum les checks cibles du scope touché
- Si une logique métier est modifiée, ajouter un test ciblé ou expliquer explicitement pourquoi il n'est pas pertinent
- Si un check est sauté, expliquer clairement pourquoi dans la réponse

## Sécurité

- Ne jamais commit de secrets ou clés API
- Utiliser des variables d'environnement via `.env` (voir `.env.example`)

<!-- nx configuration start-->
<!-- Leave the start & end comments to automatically receive updates. -->

## General Guidelines for working with Nx

- For navigating/exploring the workspace, invoke the `nx-workspace` skill first - it has patterns for querying projects, targets, and dependencies
- When running tasks (for example build, lint, test, e2e, etc.), always prefer running the task through `nx` (i.e. `nx run`, `nx run-many`, `nx affected`) instead of using the underlying tooling directly
- Prefix nx commands with the workspace's package manager (e.g., `pnpm nx build`, `npm exec nx test`) - avoids using globally installed CLI
- You have access to the Nx MCP server and its tools, use them to help the user
- For Nx plugin best practices, check `node_modules/@nx/<plugin>/PLUGIN.md`. Not all plugins have this file - proceed without it if unavailable.
- NEVER guess CLI flags - always check nx_docs or `--help` first when unsure

## Scaffolding & Generators

- For scaffolding tasks (creating apps, libs, project structure, setup), ALWAYS invoke the `nx-generate` skill FIRST before exploring or calling MCP tools

## When to use nx_docs

- USE for: advanced config options, unfamiliar flags, migration guides, plugin configuration, edge cases
- DON'T USE for: basic generator syntax (`nx g @nx/react:app`), standard commands, things you already know
- The `nx-generate` skill handles generator discovery internally - don't call nx_docs just to look up generator syntax

<!-- nx configuration end-->
