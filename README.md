# nvbes

nvbes est une entité destinée à offrir des **outils et des logiciels aux développeurs**, le tout en **open-source**, financé par son futur cloud.

Pour démarrer de façon agile et pragmatique, le projet adopte une approche de **"small building"** avec une **V1 ultra-restreinte** centrée sur le service d'authentification et de compte développeur.

## Direction V1 Ultra-Restreinte (Small Building)

La V1 sert exclusivement de service d'identité et de compte pour les développeurs utilisant les outils Nvbes (avec configuration interne et manuelle des clients OAuth) :

- **Une API monolithique** (`apps/api`) :
  - Création de compte, Connexion et maintien de session, le tout en **OAuth 2** (PKCE, RFC 6749) ;
  - Gestion des consentements (scopes et autorisations accordées par le développeur) ;
  - Profil utilisateur et sessions actives.
- **Un site web monolithique** (`apps/identity-web` / `apps/account-web` unifiés) :
  - Page de création de compte & connexion en OAuth 2 ;
  - Écran de validation du consentement OAuth 2 ;
  - Page de profil développeur (informations personnelles, sessions) ;
  - **Zéro front-end marketing** : interface purement fonctionnelle dev-first.
- **Deux Principes Majeurs** :
  1. **Rentabilité maximale & technologies les moins chères du marché** : Exploiter au maximum la gratuité et l'ultra-légèreté (Rust/Axum < 20Mo RAM, Cloudflare Pages gratuit pour le front, PostgreSQL managé ou low-cost).
  2. **Migration vers l'Open Source auto-géré** : Chaque brique est conçue pour permettre une migration transparente vers des solutions open-source (OSS) auto-hébergées dans le futur (PostgreSQL natif, Nginx/Caddy, zéro lock-in propriétaire).
- **Périmètre gelé / hors V1 immédiate** :
  - ❌ Pas de **Billing (Stripe)** pour l'instant (activé avec le cloud payant) ;
  - ❌ Pas de **Trust/Risk engine** ni de workers asynchrones complexes distribués ;
  - ❌ Pas de **front-end marketing** ni de gestion multi-tenant d'organisations complexes ;
  - ❌ **Souveraineté et conformité avancée non exigées en V1** (anticipées dans les abstractions sans complexité prématurée) ;
- **Plafond FinOps** : Coût récurrent d'infrastructure strictement plafonné à **30 EUR TTC par mois** (cible opérationnelle à 20 EUR).
- **Opérateur solo** : Administration, support et modération manuels par défaut.
- **Zéro dette technique livrée** : Code propre, sans rustines ni double source de vérité.

La [direction produit V1](docs/product/nvbes-product-strategy.md) est la source
de vérité produit. La [roadmap](docs/roadmap.md) définit les jalons d'exécution.

## État du monorepo

Le dépôt est orchestré avec pnpm workspaces, Cargo workspace et Nx. Le socle applicatif est structuré en **Monolithe Modulaire HTTP serverless** (`apps/api`) et **Worker d'arrière-plan centralisé** (`apps/worker`), complété par les interfaces web hébergées (`identity-web`, `account-web`).

```text
nvbes/
├── apps/
│   ├── api/                  # API Monolithe Modulaire HTTP serverless (Axum)
│   ├── worker/               # Worker d'arrière-plan unifié (outbox/purges)
│   ├── account-web/          # Interface web Compte & Équipes (React/Vite)
│   └── identity-web/         # Interface web Authentification & Sessions (React/Vite)
├── libs/
│   ├── rust/                 # Domaines (identity, account, billing, consent...), ports et primitives
│   └── ts/                   # SDKs, clients et runtime web réutilisables
├── contracts/                # OpenAPI, Protobuf, événements et GraphQL
├── infrastructure/           # IaC OpenTofu/Terraform et contrat FinOps
├── deploy/                   # Manifestes de déploiement
├── docs/                     # Spécifications, architecture, ADRs et opérations
└── archive/                  # Services historiques et prototypes archivés
```

Les applications Account, Cloud, Developer, Enterprise et Backoffice archivées
servent d'inventaire et de preuve historique. Toute réintroduction doit respecter
la direction V1 et faire l'objet d'une conception propre ; l'archive n'est pas
une base de production implicite.

## Stack

| Domaine         | Technologies                                        |
| --------------- | --------------------------------------------------- |
| Backend         | Rust, Axum, SQLx, Tokio, PostgreSQL                 |
| Frontend et SDK | TypeScript, React, Vite, TanStack, Effect           |
| Monorepo        | pnpm workspaces, Cargo workspace, Nx                |
| Identity        | OAuth2/OIDC, DPoP, WebAuthn, PKCE                   |
| Infrastructure  | Scaleway serverless, Cloudflare, OpenTofu/Terraform |
| Observabilité   | OpenTelemetry, Grafana Cloud, Sentry                |

## Démarrage

Prérequis : Node.js 24+, pnpm 11+, Rust stable, Docker Compose et OpenTofu.

```bash
pnpm install
pnpm env:sync
pnpm env:check
pnpm dev              # Lance l'infrastructure locale, l'API et le Worker
```

Commandes principales :

```bash
pnpm dev:api          # API Monolithique Modulaire (Axum, port 8080)
pnpm dev:worker       # Worker unifié (outbox/purges, port 8081)
pnpm dev:identity-web # Interface web Identity (Vite)
pnpm dev:account-web  # Interface web Account (Vite)
pnpm dev:infra        # Conteneurs d'infrastructure locale (PostgreSQL, Mailpit)
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

## Contribuer et signaler un problème

Les contributions sont bienvenues lorsqu'elles respectent la direction V1, les
frontières de domaine et le budget FinOps. Consulter
[`CONTRIBUTING.md`](CONTRIBUTING.md), [`GOVERNANCE.md`](GOVERNANCE.md) et le
[`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md) avant d'ouvrir une pull request.

Une vulnérabilité ne doit jamais être publiée dans une issue. Utiliser le canal
privé décrit dans [`SECURITY.md`](SECURITY.md).

## Licence

Le dépôt est distribué par défaut sous `AGPL-3.0-only`. Le SDK Rust backend
Identity déclare une exception `MIT`. La portée exacte, les textes applicables
et les règles relatives aux contenus tiers sont documentés dans la
[politique de licence](docs/legal/open-source-licensing.md).
