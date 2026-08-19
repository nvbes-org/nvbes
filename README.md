# nvbes

nvbes est une factory de micro-SaaS EU-first.

Le premier produit est nvbes Drive: un drive cloud europeen securise pour petites equipes.

## Monorepo V1

Structure runtime cible:

- `apps/cloud-web`: frontend React + Vite + TypeScript pour Cloud/Drive.
- `apps/account-web`: frontend React + Vite + TypeScript pour Account.
- `apps/console-web`: frontend React + Vite + TypeScript pour Developer.
- `apps/backoffice-web`: frontend React + Vite + TypeScript pour Backoffice.
- `apps/cloud-service`: backend Rust Cloud/Drive.
- `apps/account-service`: backend Rust Account.
- `apps/billing-service`: backend Rust Billing.
- `apps/developer-service`: backend Rust Developer.
- `apps/enterprise-service`: backend Rust Enterprise.
- `apps/gateway-cloud`: gateway Cloud.
- `apps/cloud-worker`: worker asynchrone Cloud.
- `apps/account-worker`: worker asynchrone Account.
- `apps/billing-worker`: worker asynchrone Billing.
- `libs/rust/core`: primitives Rust partagees.
- `libs/ts/http-client`, `libs/ts/identity-client`, `libs/ts/web-runtime`: runtime frontend partage.
- `infrastructure`: socle IaC et overlays d'environnements.
- `docs`: documentation produit, architecture, legal et testing.

## Orchestration Nx

Nx est utilisé comme couche de graphe projet et de ciblage des tâches au-dessus de `pnpm` et Cargo.

Commandes utiles:

- `pnpm nx:show-projects`
- `pnpm nx:show-project <project>`
- `pnpm nx:affected`

## Conventions pour agents et LLMs

- Lire [AGENTS.md](AGENTS.md) avant toute modification.
- Les fichiers Rust doivent rester plats, en dot-notation, et sous les seuils de taille définis dans [AGENTS.md](AGENTS.md).

## Demarrage local

Prerequis:

- Node.js 25+
- pnpm 11+
- Rust stable recent
- OpenTofu 1.6+ pour les validations IaC

Commandes:

```bash
pnpm env:sync
pnpm install
pnpm db:migrate
pnpm dev
```

`.env.example` est le contrat versionné et documenté. La commande crée ou
réaligne le `.env` local sans écraser ses valeurs existantes; `pnpm env:check`
détecte ensuite les variables manquantes, inconnues ou dupliquées.

Commandes utiles:

```bash
pnpm dev:web
pnpm dev:api
pnpm dev:account-worker
pnpm dev:cloud-worker
pnpm dev:cloud-db:reset
```

Conventions de scripts:

- `pnpm dev`: lance les runtimes locaux principaux.
- `pnpm dev:web`: lance les frontends locaux.
- `pnpm dev:api`: lance les services Rust locaux.
- `pnpm dev:account-service` / `pnpm dev:cloud-service`: lancent une API ciblee.
- `pnpm dev:account-worker` / `pnpm dev:cloud-worker`: lancent un worker cible.
- `pnpm dev:cloud-db:reset`: recree la base Cloud locale quand une migration dev a change.
- `pnpm db:migrate`: applique les migrations locales configurees.
- `pnpm generate:openapi`: regenere les specs OpenAPI et le SDK core.
- `pnpm format`: reformate TypeScript, JSON, Markdown et Rust.
- `pnpm format:check`: verifie le format sans modifier les fichiers.
- `pnpm lint`: lint frontend et `cargo clippy`.
- `pnpm check`: validation structure LLM-friendly, typecheck frontend et `cargo check`.
- `pnpm test`: tests unitaires et integration executable.
- `pnpm test:smoke`: smoke tests contre un environnement deploye.
- `pnpm test:e2e:critical`: E2E critiques contre un environnement deploye.
- `pnpm release:gate:staging`: gate de release staging.
- `pnpm release:gate:production`: gate de promotion production.
- `pnpm verify`: enchaine format, lint, checks et tests locaux.

## Direction Produit

- Infrastructure et residence des donnees en Europe.
- RGPD by design.
- Securite par defaut sur chaque produit.
- Modules plateforme reutilisables pour les futurs SaaS.
- Pricing par package avec extension pay-as-you-use.
- nvbes Account est la fondation IdP / Authorization Server multi-tenant pour les produits du groupe.

## Documentation

- [PRD V1](docs/product/nvbes-drive-v1-prd.md)
- [Monorepo agentic refactor workplan](docs/blueprint/nvbes-monorepo-agentic-refactor.work.md)
- [Nx workspace and dependency graph](docs/architecture/nx-workspace.md)
- [Architecture technique](docs/architecture/technical-architecture.md)
- [Architecture Identity](docs/architecture/identity-product.md)
- [Infrastructure, Network et DevOps](docs/architecture/infrastructure-devops.md)
- [Modèle de données](docs/architecture/data-model.md)
- [Contrats API V1](docs/api/v1-contracts.md)
- [API Publique V1](docs/api/public-api-v1.md)
- [Beta readiness](docs/product/beta-readiness.md)
- [Runbooks incidents](docs/operations/incident-runbooks.md)
- [Roadmap](docs/roadmap.md)

## Gouvernance Documentaire

- Les decisions structurantes sont documentees dans `docs/adr/`.
- Chaque document produit ou architecture doit distinguer les decisions acceptees des hypotheses.
- Les changements qui affectent les scripts, la structure du repo ou les conventions d’agent doivent mettre à jour `AGENTS.md` et `.codex/instructions.md`.
