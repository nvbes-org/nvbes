# nvbes

nvbes construit d'abord un socle de services réutilisables pour ses futurs
produits numériques. La V1 actuelle n'est ni une release Cloud/Drive, ni une
offre B2B : elle valide la plateforme commune avant de sélectionner le premier
produit final.

## Direction V1

- public cible B2C, tout public, avec collaboration en équipe de toute taille ;
- services mutualisés : Identity, Account, Billing, Email et Trust/Risk ;
- Platform Operations minimal pour l'administration, le support, les abus, la
  modération et FinOps ;
- un seul opérateur, avec traitement manuel par défaut ;
- coût récurrent total limité à 30 EUR TTC par mois, cible à 20 EUR ;
- aucune dette technique ou structurelle connue dans le périmètre livré ;
- mise en production progressive, mesurée, réversible et restaurable ;
- B2B, Enterprise et conformité avancée préparés dans les frontières, mais
  reportés à une version financée ultérieurement.

Cloud/Drive et les autres concepts présents dans les anciens plans restent des
hypothèses de produits futurs. Leur présence dans le dépôt ne les rend pas
prioritaires.

La [direction produit V1](docs/product/nvbes-product-strategy.md) est la source
de vérité. La [roadmap](docs/roadmap.md) définit l'ordre de livraison et les
NO-GO.

## État du monorepo

Le dépôt est orchestré avec pnpm workspaces, Cargo workspace et Nx.

```text
nvbes/
├── apps/
│   ├── email-worker/         # Runtime Email actif
│   └── trust-risk-service/   # Runtime Trust/Risk actif
├── libs/
│   ├── rust/                 # Domaines, ports, adaptateurs et primitives partagés
│   └── ts/                   # SDK, clients et runtime web réutilisables
├── contracts/                # OpenAPI, Protobuf, événements et GraphQL
├── infrastructure/           # IaC et contrat FinOps
├── deploy/                   # Manifestes de déploiement
├── docs/                     # Sources produit, architecture et opérations
└── archive/                  # Produits et prototypes retirés du runtime actif
```

Les applications Account, Cloud, Developer, Enterprise et Backoffice archivées
servent d'inventaire et de preuve historique. Toute réintroduction doit respecter
la direction V1 et faire l'objet d'une conception propre ; l'archive n'est pas
une base de production implicite.

## Stack

| Domaine | Technologies |
| --- | --- |
| Backend | Rust, Axum, SQLx, Tokio, PostgreSQL |
| Frontend et SDK | TypeScript, React, Vite, TanStack, Effect |
| Monorepo | pnpm workspaces, Cargo workspace, Nx |
| Identity | OAuth2/OIDC, DPoP, WebAuthn, PKCE |
| Infrastructure | Scaleway serverless, Cloudflare, OpenTofu/Terraform |
| Observabilité | OpenTelemetry, Grafana Cloud, Sentry |

## Démarrage

Prérequis : Node.js 24+, pnpm 11+, Rust stable, Docker Compose et OpenTofu.

```bash
pnpm install
pnpm env:sync
pnpm env:check
pnpm dev              # Email + Trust/Risk + Identity + Account + Billing
```

Commandes principales :

```bash
pnpm dev:identity-service      # Identity seul
pnpm dev:account-service       # Account seul
pnpm dev:billing-service       # Billing seul
pnpm dev:email-worker          # Email seul
pnpm dev:trust-risk-service    # Trust/Risk seul
pnpm check            # Gates complets, dont FinOps
pnpm check:finops     # Budget et plafonds serverless
pnpm lint             # Lint web et Rust
pnpm test             # Tests du workspace
pnpm verify           # Validation pré-commit
pnpm doc:validate     # Validation documentaire
pnpm agent:doctor     # Cohérence des instructions agents
```

## Règles de contribution

- lire [AGENTS.md](AGENTS.md) avant toute modification ;
- ne pas introduire de dette provisoire sur les zones touchées ;
- mesurer le coût récurrent et vérifier le budget avant toute ressource ou
  automatisation nouvelle ;
- privilégier une opération manuelle sûre tant qu'aucune automatisation conforme
  aux exigences et au budget n'existe ;
- utiliser des contrats explicites et préserver les frontières des services ;
- exécuter les checks ciblés du périmètre modifié.

## Documentation de référence

- [Direction produit V1](docs/product/nvbes-product-strategy.md)
- [Roadmap et NO-GO](docs/roadmap.md)
- [Conception FinOps et Platform Operations](docs/superpowers/specs/2026-08-25-nvbes-v1-finops-platform-operations-design.md)
- [Registre d'exécution FinOps](docs/superpowers/plans/2026-08-25-nvbes-v1-finops-foundation.md)
- [Architecture technique](docs/architecture/technical-architecture.md)
- [Infrastructure et DevOps](docs/architecture/infrastructure-devops.md)
- [Runbooks d'incidents](docs/operations/incident-runbooks.md)
- [Runbook opérateur solo](docs/operations/platform-operations-manual-runbook.md)
- [Catalogue généré des runtimes](docs/generated/catalogs/services-catalog.md)
- [Décisions d'architecture](docs/adr/)

Les documents Drive, Cloud, Developer et Enterprise doivent afficher un statut
« futur », « historique » ou « remplacé » avant d'être utilisés comme contexte
d'implémentation.
