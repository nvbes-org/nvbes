# nvbes

nvbes est une factory de micro-SaaS EU-first.

Le premier produit est nvbes Drive: un drive cloud europeen securise pour petites equipes.

## Monorepo V1

Structure initiale:

- `apps/drive-web`: frontend React + Vite + TypeScript pour Drive.
- `apps/identity-web`: frontend React + Vite + TypeScript pour Identity.
- `apps/drive-api`: backend Rust Drive.
- `apps/identity-api`: backend Rust Identity.
- `apps/drive-worker`: worker asynchrone Drive.
- `apps/identity-worker`: worker asynchrone Identity.
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
- Les règles complémentaires pour Copilot sont dans [.github/copilot-instructions.md](.github/copilot-instructions.md).
- Les règles Cursor sont dans [.cursor/README.md](.cursor/README.md) et [.cursor/rules/core-workflow.mdc](.cursor/rules/core-workflow.mdc).
- Les fichiers Rust doivent rester plats, en dot-notation, et sous les seuils de taille définis dans [AGENTS.md](AGENTS.md).

## Demarrage local

Prerequis:

- Node.js 25+
- pnpm 11+
- Rust stable recent
- OpenTofu 1.6+ pour les validations IaC

Commandes:

```bash
cp .env.example .env
pnpm install
pnpm db:migrate
pnpm dev
```

Commandes utiles:

```bash
pnpm dev:web
pnpm dev:api
pnpm dev:identity-worker
pnpm dev:drive-worker
pnpm dev:drive-db:reset
```

Conventions de scripts:

- `pnpm dev`: lance les deux APIs Rust, le worker Identity historique et les deux frontends.
- `pnpm dev:web`: lance les deux frontends.
- `pnpm dev:api`: lance les deux APIs Rust et le worker Identity historique.
- `pnpm dev:identity-api` / `pnpm dev:drive-api`: lancent une API ciblee.
- `pnpm dev:identity-worker` / `pnpm dev:drive-worker`: lancent un worker cible.
- `pnpm dev:drive-db:reset`: recree la base Drive locale quand une migration dev a change.
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
- nvbes Identity est la fondation IdP / Authorization Server multi-tenant pour les produits du groupe.

## Documentation

- [PRD V1](docs/prd/nvbes-drive-v1.md)
- [Monorepo agentic refactor workplan](docs/blueprint/nvbes-monorepo-agentic-refactor.work.md)
- [Nx workspace and dependency graph](docs/architecture/nx-workspace.md)
- [Architecture technique](docs/architecture/technical-architecture.md)
- [Architecture Identity](docs/architecture/identity-product.md)
- [Infrastructure, Network et DevOps](docs/architecture/infrastructure-devops.md)
- [Modele de donnees](docs/domain/data-model.md)
- [Contrats API V1](docs/api/v1-contracts.md)
- [API Publique V1](docs/api/public-api-v1.md)
- [Beta readiness](docs/beta/beta-readiness.md)
- [Runbooks incidents](docs/operations/incident-runbooks.md)
- [Roadmap](docs/roadmap.md)

## Gouvernance Documentaire

- Les decisions structurantes sont documentees dans `docs/adr/`.
- Chaque document produit ou architecture doit distinguer les decisions acceptees des hypotheses.
- Les changements de pricing, billing, securite, infra et conformite doivent etre relus avant implementation.
- Les changements qui affectent les scripts, la structure du repo ou les conventions d’agent doivent mettre à jour `AGENTS.md` et, si besoin, `.github/copilot-instructions.md` et les règles Cursor.
