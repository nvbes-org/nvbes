# nvbes

**nvbes** est une factory de micro-SaaS souverains, sécurisés et européens (EU-first).

L'écosystème comprend des produits autonomes et modulaires bâtis sur un socle multi-tenant unifié :
- **nvbes Drive / Cloud** : Stockage cloud d'équipe sécurisé, souverain et chiffré.
- **nvbes Identity / Account** : Fournisseur d'identité (IdP) et serveur d'autorisation OAuth2 / OIDC / FAPI, WebAuthn & Passkeys.
- **nvbes Developer / Console** : Portail développeurs pour la gestion d'applications, API keys et webhooks.
- **nvbes Billing** : Moteur de facturation unifié avec gestion d'abonnements et routage de paiements.
- **nvbes Enterprise & Backoffice** : Gestion multi-organisations, conformité RGPD, audit append-only et console d'administration.

---

## Architecture & Structure du Monorepo

Le monorepo est orchestré via **pnpm workspaces**, **Cargo workspace** et **Nx** :

```text
nvbes/
├── apps/
│   ├── cloud-web/            # Frontend Cloud / Drive (React 19 + Vite + Tailwind + shadcn)
│   ├── account-web/          # Frontend Account (React + Vite + shadcn)
│   ├── identity-web/         # Frontend Identity / Authentification (React + Vite)
│   ├── console-web/          # Frontend Developer Console (React + Vite)
│   ├── enterprise-web/       # Frontend Enterprise Portal (React + Vite)
│   ├── backoffice-web/       # Frontend Administration & Backoffice (React + Vite)
│   ├── cloud-service/        # API Rust Cloud / Drive (Axum + SQLx)
│   ├── account-service-next/ # API Rust Account (Axum + SQLx)
│   ├── identity-service/     # API Rust Identity / OIDC (Axum + SQLx)
│   ├── billing-service/      # API Rust Billing (Axum + SQLx)
│   ├── developer-service/    # API Rust Developer (Axum + SQLx)
│   ├── enterprise-service/   # API Rust Enterprise (Axum + SQLx)
│   ├── backoffice-service/   # API Rust Backoffice (Axum + SQLx)
│   ├── gateway-cloud/        # Gateway Cloud & Proxy Edge
│   └── *-worker/             # Workers asynchrones Rust (cloud, account, identity, billing, email)
├── libs/
│   ├── rust/                 # Primitives partagées (core, platform, ports, observability, dpop, storage...)
│   └── ts/                   # SDKs TS, clients HTTP typés, web-runtime, web-ui, design-system
├── contracts/                # Contrats d'interfaces OpenAPI, Protobuf, GraphQL, Events
├── infrastructure/           # IaC OpenTofu/Terraform, Docker Compose local & observabilité Grafana
└── docs/                     # Documentation architecture, produit, conformité, sécurité et ADRs
```

---

## Stack Technique

| Domaine | Technologies |
|---|---|
| **Backend** | Rust (Axum 0.8, SQLx 0.8, Tokio, PostgreSQL, Redis, Utoipa OpenAPI) |
| **Frontend** | TypeScript, React 19, Vite, TanStack Router / Query, Effect, Tailwind CSS, shadcn/ui |
| **Monorepo** | pnpm workspaces, Cargo workspace, Nx |
| **Sécurité & Auth** | OAuth2 / OIDC, FAPI 2.0, DPoP (RFC 9449), WebAuthn / Passkeys, PKCE |
| **Billing** | Stripe, Stripe Webhooks, routage de paiements multi-prestataires |
| **Infra & Observabilité** | Scaleway (EU), Docker Compose, OpenTofu, Grafana, Alloy, Beyla, Prometheus, Sentry |

---

## Démarrage Rapide

### Prérequis

- **Node.js** : `>= 24.0.0` (support LTS)
- **pnpm** : `>= 11.1.2`
- **Rust** : stable récent (édition 2024)
- **Docker & Docker Compose** (pour l'infrastructure locale)
- **OpenTofu** : `>= 1.6` (pour les validations IaC)

### Initialisation

```bash
# 1. Synchroniser et vérifier l'environnement
pnpm env:sync
pnpm env:check

# 2. Installer les dépendances frontend et outils
pnpm install

# 3. Démarrer l'infrastructure locale (Postgres, Redis, etc.)
pnpm dev:infra

# 4. Appliquer les migrations de base de données
pnpm db:migrate

# 5. Lancer l'environnement de développement
pnpm dev
```

---

## Commandes Principales

### Développement local

```bash
pnpm dev                     # Runtimes locaux principaux (Web + APIs)
pnpm dev:web                 # Tous les frontends
pnpm dev:api                 # Tous les services Rust
pnpm dev:cloud               # Stack Cloud / Drive (web + API)
pnpm dev:account             # Stack Account / Identity (web + API)
pnpm dev:cloud-service       # Service Cloud uniquement
pnpm dev:identity-service    # Service Identity uniquement
pnpm dev:infra               # PostgreSQL, Redis, MinIO via Docker Compose
pnpm dev:infra:obs           # Stack d'observabilité locale (Grafana, Alloy, Beyla)
```

### Qualité, Lint & Tests

```bash
pnpm check                   # Suite complète de vérifications (contrats, secrets, types, cargo check)
pnpm verify                  # Validation pré-commit complète (format, lint, check, tests)
pnpm test                    # Tests unitaires et d'intégration
pnpm test:unit               # Tests unitaires
pnpm test:integration        # Tests d'intégration Rust et IaC
pnpm test:e2e:critical       # Tests E2E critiques
pnpm lint                    # Linter web et Rust (cargo clippy)
pnpm format                  # Formatage automatique TS, JSON, Rust, Markdown
pnpm format:check            # Vérification du formatage
```

### Orchestration Nx & Génération

```bash
pnpm nx:show-projects        # Lister tous les projets du graphe Nx
pnpm nx:show-project <nom>   # Inspecter la configuration d'un projet cible
pnpm nx:affected             # Exécuter les tâches affectées par les changements en cours
pnpm generate:openapi        # Régénérer les schémas OpenAPI et les SDKs TypeScript
pnpm generate:email          # Compiler les composants et templates email
```

---

## Règles et Conventions du Codebase

Pour assurer une maintenabilité optimale par des agents IA et des développeurs :

1. **Règle V0 (Zéro Dette)** : Aucun compromis temporaire ni rustine sur les zones modifiées. Redesign privilégié.
2. **Conventions Rust** :
   - Fichiers source dans `src/` avec nommage plat en dot-notation (ex: `identity.domains.billing.service.rs`).
   - Modules déclarés explicitement via `#[path = "..."]` dans `mod.rs`.
   - Respect strict des limites de taille : **< 300 lignes** recommandé, **500 lignes** max.
3. **Conventions TypeScript** :
   - Interdiction stricte de `any` (`unknown` ou typage précis obligatoire).
4. **Hiérarchie UI Frontend** :
   - 1. Registry interne (`libs/ts/web-ui`, `apps/*/components/ui`)
   - 2. Registry officiel shadcn/ui (`pnpm dlx shadcn@latest add <component>`)
   - 3. Registries externes compatibles
   - 4. Tailwind CSS (classes utilitaires)
   - 5. CSS pur / modules (dernier recours uniquement)
5. **Agent OS** :
   - Lire [AGENTS.md](AGENTS.md) avant toute contribution.
   - Instructions et mémoire synchronisées depuis `docs/agent/*` et `.codex/instructions.md`.

---

## Documentation de Référence

- **Produit** :
  - [PRD V1 Drive](docs/product/nvbes-drive-v1-prd.md)
  - [Beta Readiness](docs/product/beta-readiness.md)
  - [Roadmap](docs/roadmap.md)
- **Architecture** :
  - [Architecture Technique Globale](docs/architecture/technical-architecture.md)
  - [Architecture Identity](docs/architecture/identity-product.md)
  - [Graphe Nx & Dépendances](docs/architecture/nx-workspace.md)
  - [Modèle de Données](docs/architecture/data-model.md)
  - [Runtime Frontend](docs/architecture/frontend-runtime.md)
  - [Infrastructure & DevOps](docs/architecture/infrastructure-devops.md)
- **APIs & Contrats** :
  - [Contrats API V1](docs/api/v1-contracts.md)
  - [API Publique V1](docs/api/public-api-v1.md)
- **Conformité & Opérations** :
  - [Conformité Entreprise & RGPD](docs/compliance/enterprise-compliance-summary.md)
  - [Runbooks d'Incidents](docs/operations/incident-runbooks.md)
  - [Registre ADR (Architecture Decision Records)](docs/adr/)
