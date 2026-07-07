# nvbes Project

Monorepo full-stack avec Rust (Axum), TypeScript/React, orchestré par pnpm workspaces + Cargo workspace.

## Agent contract

- Lire ce fichier avant de modifier le codebase.
- Préférer les fichiers courts, plats, et auto-documentants.
- Ne pas introduire de dette technique provisoire sur les zones touchées.
- Ne pas renommer ou déplacer des fichiers hors scope sans raison de design claire.
- Utiliser Nx pour explorer le graphe de projets, les cibles et le périmètre affecté quand cela aide à réduire le contexte.

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
│   ├── account-service/    # Service Account / identité (Axum)
│   ├── account-web/        # Frontend Account (React)
│   ├── account-worker/     # Worker asynchrone Account
│   ├── billing-service/    # Service Billing (Axum)
│   ├── billing-worker/     # Worker asynchrone Billing
│   ├── cloud-service/      # API stockage/fichiers Cloud (Axum)
│   ├── cloud-web/          # Frontend Cloud/Drive (React + shadcn/ui)
│   ├── cloud-worker/       # Worker asynchrone Cloud
│   ├── console-web/        # Frontend Developer Console
│   ├── developer-service/  # Service Developer (Axum)
│   ├── enterprise-service/ # Service Enterprise (Axum)
│   ├── enterprise-web/     # Frontend Enterprise
│   ├── gateway-cloud/      # Gateway Cloud
│   ├── backoffice-service/ # Service Backoffice (Axum)
│   └── backoffice-web/     # Frontend Backoffice
├── libs/
│   ├── rust/
│   │   ├── core/           # Primitives partagées (config, auth, mfa)
│   │   ├── audit/          # Audit append-only
│   │   ├── billing/        # Logique billing partagée
│   │   ├── email/          # Service email
│   │   ├── identity-sdk-backend/ # SDK Rust standalone
│   │   ├── observability/  # Métriques, tracing, sentry
│   │   ├── region/         # Régions et résidence des données
│   │   ├── scan/           # Scan engine
│   │   ├── storage/        # Object storage
│   │   └── tenancy/        # Multi-tenant primitives
│   └── ts/
│       ├── email-templates/    # Templates email
│       ├── identity-sdk/       # SDK TypeScript
│       ├── identity-sdk-core/  # Types OpenAPI générés
│       ├── identity-sdk-web/   # SDK web (PKCE, WebAuthn, MFA)
│       ├── http-client/        # Client HTTP TS valide runtime
│       ├── identity-client/    # Client Identity TS typé
│       └── web-runtime/        # Runtime React Query/Effect partagé
└── infrastructure/         # Terraform/Ansible
```

## Repository instructions for agents

- `.github/copilot-instructions.md` contient les règles générales du repo pour les agents GitHub/Copilot.
- `.cursor/rules/core-workflow.mdc` contient les règles globales Cursor.
- `.cursor/rules/rust-backend.mdc` couvre les conventions Rust.
- `.cursor/rules/typescript-react.mdc` couvre les conventions TypeScript/React.
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
pnpm dev:web          # Frontend uniquement
pnpm dev:api          # Services Rust
pnpm dev:account-service
pnpm dev:cloud-service
pnpm dev:account-worker
pnpm dev:cloud-worker
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

### Contexte V0

- Ce produit est en **V0**.
- À ce stade, **0 dette technique** n'est acceptable.
- Si une partie du design ou de l'implémentation est bancale, le **redesign complet** est hautement préféré et encouragé.
- Éviter les rustines, les compromis temporaires et les extensions incrémentales sur une base faible.

# Hiérarchie DE SÉLECTION DE COMPOSANTS (OBLIGATOIRE — TOUS LLM)

**Pour TOUS les sites web/projets frontend, suivre cet ordre STRICTEMENT, sans exception :**

1. **Registry du projet** — Utiliser d'abord les composants du registry interne du projet (`libs/ts/web-ui`, `apps/*/components/ui`, registry local `components.json`)
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

- `apps/account-service` est le produit Account final, pas un copié de `apps/cloud-service`
- Réécrire le code si nécessaire plutôt que de copier/coller
- Les services doivent être complets ou non-existants (pas de moitié implémenté)
- Pas de duplication de code entre `apps/cloud-service` et `apps/account-service`

### Architecture des Services

- **Services spécialisés** : Éviter les "God Objects" (un seul gros service par domaine). Préférer scinder en capacités métier (ex: `SubscriptionService`, `CheckoutService`).
- **Module-as-a-Service** : Si un service n'a pas d'état interne, privilégier des fonctions simples dans un module plutôt qu'une struct avec des méthodes.
- **Séparation des couches** : Maintenir une séparation nette entre persistence (`db.rs`), validation métier pure (`validation.rs`) et orchestration (`service.rs` ou `logic.rs`).

## Validation avant fin de tâche

- Après modification de code: lancer au minimum les checks cibles du scope touché
- Si une logique métier est modifiée, ajouter un test ciblé ou expliquer explicitement pourquoi il n'est pas pertinent
- Si un check est sauté, expliquer clairement pourquoi dans la réponse

## Sécurité

- Ne jamais commit de secrets ou clés API
- Utiliser des variables d'environnement via `.env` (voir `.env.example`)
