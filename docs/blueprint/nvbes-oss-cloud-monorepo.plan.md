# Plan Monorepo nvbes OSS et nvbes Cloud

## Objectif

Formaliser comment le monorepo prive `nvbes` gere en meme temps:

- `nvbes OSS`: distribution publique, self-hostable, 100% fonctionnelle;
- `nvbes Cloud`: SaaS manage, supporte, opere et conforme;
- les documents internes qui ne doivent jamais etre publies.

Le monorepo prive reste la source de verite. Le repository public `nvbes-oss` est genere depuis une allowlist stricte.

## Principe Central

La separation ne doit pas etre "features gratuites" contre "features payantes".

La separation correcte est:

- fonctionnalites produit: OSS;
- operations managees: Cloud;
- strategie, runbooks sensibles et business: Internal.

Regle de dependance:

```text
OSS      -> peut dependre de OSS/shared
Cloud    -> peut dependre de OSS et Cloud
Internal -> peut dependre de OSS, Cloud et Internal

OSS ne depend jamais de Cloud ou Internal.
```

## Structure Cible

```text
nvbes/
  apps/
    identity-api/              # OSS
    identity-web/              # OSS
    drive-api/                 # OSS
    drive-web/                 # OSS
    cloud-control-api/         # Cloud only
    cloud-console/             # Cloud only
    internal-admin/            # Internal only

  libs/
    rust/
      platform/                # OSS: primitives transverses
      products/                # OSS: logique metier produit
      adapters-oss/            # OSS: MinIO, SMTP, Lago, NATS, OpenSearch
      adapters-cloud/          # Cloud only: Stripe, R2, KYC provider, email provider
      cloud/                   # Cloud only: provisioning, metering, SLA, ops
    ts/
      sdk/                     # OSS
      web-ui/                  # OSS
      cloud-ui/                # Cloud only

  deploy/
    oss/                       # Docker Compose, Helm, Kustomize, OpenTofu OSS
    cloud/                     # Scaleway, AWS, Cloudflare, managed Kubernetes
    internal/                  # outils et environnements internes

  docs/
    oss/                       # exporte vers nvbes-oss/docs
    cloud/                     # docs clients Cloud
    internal/                  # jamais exporte
    blueprint/                 # strategie privee

  tools/
    oss-export/
      manifest.json
      export.mjs
      checks.mjs
```

Cette structure est cible. Elle peut etre atteinte progressivement, mais les nouvelles capacites doivent deja respecter ces frontieres.

## Regles Produit

`nvbes OSS` doit contenir 100% des fonctionnalites applicatives:

- Identity Center;
- Drive et Desktop Suite;
- workflows de billing;
- audit;
- permissions;
- integrations standard;
- installation Docker et Kubernetes.

`nvbes Cloud` peut ajouter seulement des capacites d'operation managee:

- hosting;
- provisioning tenants;
- metering;
- billing cloud;
- backups et restauration;
- monitoring;
- support;
- SLA;
- conformite;
- migrations assistees;
- integrations proprietaires optionnelles.

Une fonctionnalite produit ne doit pas etre disponible uniquement dans Cloud.

## Pattern Adapters

La logique metier ne doit pas appeler directement un provider proprietaire.

Chaque integration externe passe par une interface stable:

```text
BillingProvider
ObjectStorage
EmailProvider
KycProvider
QueueProvider
SearchProvider
AnalyticsProvider
```

Les implementations sont separees:

```text
libs/rust/adapters-oss/
  billing-lago/
  storage-minio/
  email-smtp/
  queue-nats/
  search-opensearch/

libs/rust/adapters-cloud/
  billing-stripe/
  storage-r2/
  email-provider/
  kyc-provider/
```

Le comportement produit reste identique. Seule l'implementation d'infrastructure change.

## Regles de Dependances

### TypeScript et Nx

Chaque projet doit porter des tags explicites:

```json
{
  "tags": ["scope:oss", "type:app", "product:identity"]
}
```

Regles attendues:

- `scope:oss` peut importer seulement `scope:oss`;
- `scope:cloud` peut importer `scope:oss` et `scope:cloud`;
- `scope:internal` peut importer tous les scopes;
- une importation `scope:oss -> scope:cloud` ou `scope:oss -> scope:internal` bloque la CI.

### Rust

Un check base sur `cargo metadata` doit refuser toute dependance transitive:

- d'un crate OSS vers un crate Cloud;
- d'un crate OSS vers un crate Internal;
- d'un crate exportable vers une dependance a licence non compatible OSS.

Les crates exposees dans `nvbes-oss` doivent declarer clairement leur role et leur licence.

## Export Public

Le repository public `nvbes-oss` n'est pas un miroir git.

Il est genere avec un manifeste allowlist:

```json
{
  "include": [
    "apps/identity-api",
    "apps/identity-web",
    "apps/drive-api",
    "apps/drive-web",
    "libs/rust/platform",
    "libs/rust/products",
    "libs/rust/adapters-oss",
    "libs/ts/sdk",
    "libs/ts/web-ui",
    "deploy/oss",
    "docs/oss",
    "README.oss.md",
    "LICENSE",
    "SECURITY.md",
    "CONTRIBUTING.md",
    "CODE_OF_CONDUCT.md"
  ],
  "exclude": [
    "docs/cloud",
    "docs/internal",
    "docs/blueprint",
    "deploy/cloud",
    "deploy/internal",
    "libs/rust/adapters-cloud",
    "libs/rust/cloud",
    "apps/cloud-*",
    "apps/internal-*"
  ]
}
```

Pendant l'export:

- `README.oss.md` devient `nvbes-oss/README.md`;
- `docs/oss/*` devient `nvbes-oss/docs/*`;
- aucun fichier de `docs/blueprint`, `docs/cloud` ou `docs/internal` n'est copie;
- les secrets, fichiers `.env`, runbooks prives et roadmaps internes sont exclus;
- le repo public recoit un historique propre ou des commits d'export, pas l'historique prive complet.

## Documentation

La documentation est separee par audience:

- `docs/oss`: installation, admin self-hosted, architecture OSS, upgrade, troubleshooting;
- `docs/cloud`: usage client Cloud, SLA, billing Cloud, support, regions;
- `docs/internal`: runbooks internes, incidents, procedures d'exploitation sensibles;
- `docs/blueprint`: strategie produit, plans long terme, decisions non publiques.

La documentation publique ne doit pas pointer vers les blueprints prives.

## CI Attendues

La CI du monorepo prive doit avoir deux axes.

Pour OSS:

- boundary checks Nx et Rust;
- secret scan;
- license scan OSI-only;
- audit provider OSS avec baseline explicite des fuites legacy;
- `cargo check` sur le subset OSS;
- `pnpm` checks sur le subset OSS;
- build des images Docker OSS;
- lint Helm/Kustomize/OpenTofu OSS;
- generation SBOM;
- export vers `nvbes-oss`.

Pour Cloud:

- checks complets du monorepo;
- build des apps OSS et Cloud;
- tests des adapters cloud;
- validation des manifests cloud;
- deploiement vers les environnements manages.

## Criteres d'Acceptation

Le modele est correctement applique quand:

- une feature produit peut etre lancee en OSS sans dependance Cloud;
- le Cloud peut remplacer les adapters OSS par des adapters manages;
- un export public ne contient aucun document interne;
- les docs OSS et Cloud ont des audiences distinctes;
- la CI bloque toute dependance OSS vers Cloud/Internal;
- le business model repose sur l'operation managee, pas sur une limitation artificielle des features.

## Decisions Appliquees

- Politique de licence initiale: AGPL-3.0-only pour la distribution OSS principale, avec exception MIT existante pour le SDK Rust genere.
- Script d'export `tools/oss-export` avec manifeste allowlist, checks et export local.
- Checks de frontieres Nx et Rust integres aux scripts racine.
- Audit provider OSS pour bloquer tout nouveau couplage Stripe/Sentry/Scaleway/Cloudflare/PostHog hors baseline.
- Email OSS branche sur SMTP, avec adapter Scaleway isole sous `libs/rust/adapters-cloud`.
- Analytics Rust OSS expose un sink generique, avec adapter PostHog isole sous `libs/rust/adapters-cloud`.
- Documentation publique OSS, Cloud et Internal separee par audience.
- Squelette de deploiement OSS sous `deploy/oss`.

## Decisions Restantes

- Revue juridique finale des licences par crate et package, notamment pour les SDKs.
- Creation effective du repository public `nvbes-oss`.
- Refactor physique des dossiers existants vers la structure cible.
- Extraction progressive des providers legacy vers `adapters-oss` et `adapters-cloud`.
- Publication des images Docker et charts Helm OSS.
