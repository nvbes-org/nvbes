# Plan Structuration Complete Big Bang Zero Dette

## Objectif

Documenter une reconstruction complete de nvbes depuis zero, sans prendre la
stack actuelle comme reference technique, puis migrer l'ensemble du projet dans
une bascule Big Bang maitrisee.

Ce plan est prive. Il formalise la cible strategique, les decisions
d'architecture et les regles de qualite. Le runbook operationnel de migration
est dans [Runbook Big Bang](../migration/nvbes-big-bang-migration-runbook.md).

## Principe Central

La migration ne doit pas etre une suite de rustines sur l'existant.

Construire une nouvelle plateforme propre en parallele, valider la parite
fonctionnelle et les donnees, puis basculer en une seule fenetre controlee.

Regles non negociables:

- aucune dette temporaire acceptee au cutover;
- aucun provider proprietaire dans le core produit;
- aucune dependance OSS vers Cloud ou Internal;
- aucun endpoint public sans contrat;
- aucun evenement critique sans schema versionne;
- aucune migration de donnees sans reconciliation;
- aucun composant runtime legacy apres decommission.

## Stack Cible

| Zone | Choix |
|---|---|
| Backend core | Rust |
| Control plane cloud | Go |
| Frontend, SDK, tooling | TypeScript |
| Data, AI, OCR, analytics avance | Python |
| API publique | REST + OpenAPI |
| API interne | gRPC + Protobuf |
| Async durable | Kafka ou Redpanda + Outbox + Schema Registry |
| Frontend | React + TanStack Router/Query/Table/Form/Virtual |
| UI | shadcn/ui + Radix/Base UI + Tailwind |
| OLTP initial | PostgreSQL par cellule |
| Hot path massif | ScyllaDB |
| Cache/session/rate-limit | Valkey |
| Analytics/logs/events | ClickHouse |
| Lakehouse | Iceberg + Trino |
| Search | OpenSearch |
| Object storage | S3-compatible |
| Edge/light APIs | Hono |
| Orchestration finale | Kubernetes + Cilium + Envoy Gateway |

## Structure Monorepo Cible

```text
nvbes/
  apps/
    public/                  # frontends publics et produits
    api/                     # APIs REST publiques et gateways
    workers/                 # workers metier et jobs critiques
    cloud/                   # control plane et console Cloud
    internal/                # outils internes non exportables

  libs/
    rust/
      platform/              # primitives transverses
      products/              # logique metier produit
      ports/                 # interfaces infrastructure
      adapters-oss/          # implementations OSS
      adapters-cloud/        # implementations Cloud
      cloud/                 # provisioning, metering, SLA, ops
    go/
      control-plane/
      kubernetes-operators/
      provisioning/
      network/
    ts/
      sdk-core/
      sdk-identity/
      sdk-drive/
      http-client/
      web-runtime/
      web-ui/
      design-system/
      edge-runtime/
      cloud-ui/
    python/
      analytics-pipelines/
      ai-services/
      ocr/
      data-quality/

  contracts/
    openapi/
    protobuf/
    events/

  deploy/
    oss/
    cloud/
    internal/

  docs/
    oss/
    cloud/
    internal/
    blueprint/
    adr/
    migration/

  tools/
    codegen/
    oss-export/
    boundary-checks/
    migration/
    load-tests/
    security/
```

## Frontieres de Domaine

Chaque domaine doit etre reconstruit comme unite autonome avec modele,
commandes, requetes, evenements, ports et policies.

Domaines de base:

- Identity: comptes, sessions, MFA, WebAuthn, OAuth2/OIDC, SAML, SCIM,
  service accounts.
- Workspace: tenants, organizations, memberships, roles, policies.
- Drive: metadata fichiers, dossiers, upload, download, partage, quotas.
- Billing: plans, entitlements, usage ledger, invoices, webhooks.
- Audit: journal append-only, exports, conformite.
- Privacy: export/suppression RGPD, consentements, retention.
- Developer Platform: apps, tokens, webhooks, API docs, SDKs.
- Cloud: provisioning, metering, billing cloud, SLA, support, operations.

Regles:

- `apps/*` contient seulement bootstrap, routing, handlers et composition.
- la logique metier vit dans `libs/*/products`;
- les APIs exposent des DTOs separes du modele domaine;
- un domaine ne lit pas directement la base d'un autre domaine;
- toute integration externe passe par un port puis un adapter.

## Contrats

### API Publique

- REST JSON.
- OpenAPI obligatoire.
- Versioning `/v1`.
- `Idempotency-Key` obligatoire sur mutations critiques.
- Pagination cursor-based.
- Erreur standardisee: `code`, `message`, `request_id`, `details`.
- SDKs generes TypeScript, Rust et Go.

### API Interne

- gRPC + Protobuf.
- Deadlines obligatoires.
- Retry policy controlee.
- mTLS service-to-service.
- Compatibilite des `.proto` testee en CI.

### Events

Enveloppe commune:

```json
{
  "event_id": "uuid",
  "event_type": "identity.user.created",
  "event_version": 1,
  "tenant_id": "uuid",
  "region_id": "eu-fr",
  "occurred_at": "iso8601",
  "correlation_id": "uuid",
  "idempotency_key": "string",
  "payload": {}
}
```

Tout evenement critique doit avoir:

- schema versionne;
- publication via outbox;
- idempotence consumer;
- retry;
- DLQ;
- capacite de replay.

## Data Architecture

Sources de verite:

- PostgreSQL: transactions metier, identity, billing, workspace, drive metadata.
- Object storage S3-compatible: fichiers, exports, medias, backups, artefacts.
- Valkey: cache, sessions courtes, rate-limit, locks legers.
- Kafka/Redpanda: journal durable d'evenements.
- ClickHouse: logs, analytics, audit exploratoire, usage, FinOps.
- OpenSearch: recherche documents, audit, mail, logs exploratoires.
- ScyllaDB: hot paths massifs uniquement.
- Iceberg: datasets long terme, DaaS, public data, historique analytics.

Regles de donnees:

- `tenant_id`, `region_id`, `data_residency`, `created_at`, `updated_at`
  sur les tables critiques;
- migrations SQL versionnees;
- pas d'analytics lourd sur OLTP;
- pas de donnees personnelles sensibles dans les events sans classification;
- pas de stockage long terme des artefacts bruts KYC;
- object keys opaques;
- backups chiffres et restauration testee.

## Frontend

Stack verrouillee:

- React;
- TanStack Router pour routing;
- TanStack Query pour etat serveur;
- TanStack Table, Form et Virtual selon besoin;
- Zustand uniquement pour etat UI complexe;
- Zod pour validation runtime;
- shadcn/ui + Radix/Base UI avant custom;
- Tailwind CSS;
- Playwright et Vitest.

Regles:

- pages fines;
- logique dans hooks, modules domaine ou SDK;
- aucun fetch direct hors clients SDK;
- aucun etat serveur dans Zustand;
- aucun `any`;
- accessibilite WCAG AA sur les parcours cles.

## OSS, Cloud et Internal

```text
OSS      -> fonctionnalites applicatives completes
Cloud    -> hebergement manage, provisioning, metering, support, SLA, ops
Internal -> strategie, runbooks sensibles, admin interne
```

OSS contient toutes les fonctionnalites produit. Cloud ajoute seulement les
capacites d'operation managee et les adapters proprietaires optionnels.

L'export public `nvbes-oss` reste genere depuis une allowlist stricte. Il ne
doit jamais inclure `docs/blueprint`, `docs/migration`, `docs/internal`,
`deploy/cloud`, `deploy/internal`, `adapters-cloud`, secrets ou runbooks prives.

## Phases de Reconstruction

1. Freeze et inventaire: endpoints, tables, events, jobs, docs, infra, secrets.
2. Nouvelle fondation: workspaces Rust, Go, TS, Python, CI, checks et contracts.
3. Primitives plateforme: config, errors, tenancy, audit, outbox, ports, observability.
4. Identity: auth, sessions, MFA, OAuth, SAML/SCIM scaffold, service accounts.
5. Workspace/Authz: tenants, organizations, roles, policies et decisions.
6. Drive: metadata, upload, download, partage, quotas, privacy.
7. Billing/Usage: plans, entitlements, ledger, metering, webhooks.
8. Developer Platform: apps, tokens, webhooks, docs API, SDKs.
9. Frontends: Identity, Drive, Developer, Cloud Console, Internal Admin.
10. Infra/deploy: OCI, Helm, OpenTofu, secrets, backups, observability.
11. Repetitions migration: dry runs, dress rehearsal, reconciliation.
12. Big Bang cutover: read-only, export final, import, smoke, DNS, monitoring.
13. Decommission: suppression legacy, archivage, audit post-migration.

## Gates de Decision

Chaque phase se termine par une revue explicite. Une phase suivante peut
demarrer en parallele seulement si les contracts et boundaries de la phase
precedente sont stables.

| Gate | Condition go | Condition no-go |
|---|---|---|
| G0 Freeze | inventaire signe, scope gele, owners nommes | element runtime sans decision |
| G1 Fondation | CI, workspaces et boundaries actifs | import interdit non bloque |
| G2 Primitives | audit, outbox, tenancy, errors et observability testes | mutation critique sans idempotence |
| G3 Domaines | parite fonctionnelle et contracts publics valides | domaine sans migration data |
| G4 Frontends | parcours critiques E2E et accessibilite AA | fetch direct ou `any` restant |
| G5 Infra | staging recree from scratch, backup/restore valide | rollback non teste |
| G6 Repetitions | deux reconciliations consecutives stables | ecart data non explique |
| G7 Cutover | smoke, SLO, logs et rollback OK | corruption ou auth globale KO |
| G8 Decommission | legacy inactif, secrets retires, audit approuve | runtime legacy encore requis |

## Backlog Executable

### Statut d'Execution - 2026-06-17

Etat actuel: fondation documentaire et controles CI initiaux en place. La
structuration n'est pas encore eligible au cutover production.

### Statut d'Execution - 2026-07-05

Le refactor Account/Cloud Big Bang est le lot actif de taxonomie runtime. Le
scope gele, les noms legacy, les noms cibles, les owners, les sources de
donnees, les contracts, les jobs, les variables d'environnement, les images et
l'observabilite sont recenses dans
[Inventaire Account Cloud Big Bang](../migration/account-cloud-big-bang.inventory.md).

Decision de taxonomie: les anciens runtimes `identity-*` deviennent Account,
les anciens runtimes `drive-*` deviennent Cloud, `billing-api` devient
`billing-service`, `developer-web` devient `console-web`, `internal-admin*`
devient Backoffice, et `gateway-graphql` devient `gateway-cloud`. Aucun alias de
compatibilite runtime ne doit survivre au cutover.

Termine cote repository:

- ADR clean rebuild et boundaries OSS/Cloud/Internal;
- manifestes de contrats OpenAPI, Protobuf et events;
- inventaire genere des endpoints et tables source;
- mappings generes pour donnees, secrets, jobs, ressources et risques;
- template de reconciliation et schema de rapport;
- evidence gates G0 a G8 initialisee en no-go;
- checks CI pour contracts, boundaries, migration artifacts, secrets et export OSS.

Verification locale:

- `pnpm check` passe le 2026-06-17, avec warnings de taille de fichiers
  existants sous le seuil bloquant de 500 lignes;
- `pnpm check:migration-precutover -- --env production --reconciliation-report
  docs/migration/reconciliation.<run>.json` est le gate strict avant cutover;
- `tools/migration/gate-evidence.mjs --strict` echoue volontairement tant que
  les gates n'ont pas owners, preuves et decision `go`.

Bloqueurs restants avant cutover:

- owners reels a nommer pour chaque gate, domaine, data set, secret et job;
- decisions `keep/rebuild/drop/replace` a signer pour chaque table, secret, job,
  ressource et risque;
- implementation cible complete a construire pour les domaines, frontends,
  infra et operations;
- trois repetitions documentees avec reconciliation stable;
- rollback chronometre, preuves attachees et audit post-migration approuve.

### 0. Gouvernance

- creer ADR clean rebuild et boundaries OSS/Cloud/Internal;
- nommer owners par domaine, contrat, donnees et infra;
- definir severites bloquantes, SLA de correction et calendrier freeze;
- ouvrir un registre des risques avec mitigation et owner.

### 1. Inventaire Source

- exporter la liste des endpoints publics, routes internes et jobs;
- inventorier tables, migrations, buckets, queues, topics, secrets et cron;
- produire une matrice garder/reconstruire/supprimer/remplacer;
- documenter chaque source de donnees avec volume, criticite et retention.

### 2. Fondation Technique

- creer la nouvelle structure monorepo et les projets Nx/Cargo/Go/Python;
- ajouter checks boundaries TypeScript, Rust et export OSS;
- ajouter contrats OpenAPI, Protobuf et events avec generation SDK;
- ajouter scans secrets, licences, SBOM, images et dependances.

### 3. Produits et Donnees

- reconstruire Identity puis Workspace/Authz comme base des autres domaines;
- reconstruire Drive, Billing, Audit, Privacy et Developer Platform;
- fournir pour chaque domaine export, transform, import et reconciliation;
- ajouter tests unitaires, integration, contract tests et parcours E2E.

### 4. Operations et Migration

- deployer staging from scratch avec donnees anonymisees;
- automatiser snapshots, restore, migration et rollback;
- executer trois repetitions documentees;
- executer cutover Big Bang seulement si tous les gates sont verts.

## Definition de Done Domaine

Un domaine est termine quand:

- son modele domaine ne depend pas des DTOs HTTP ni d'un provider concret;
- ses commandes critiques sont idempotentes et auditees;
- ses events critiques passent par outbox et schema versionne;
- ses tables portent `tenant_id`, `region_id` et timestamps quand applicable;
- ses migrations data ont export, transform, import, reject log et checksum;
- ses endpoints publics sont couverts par OpenAPI et SDK genere;
- ses invariants metier ont tests automatises;
- ses erreurs utilisent l'enveloppe standardisee.

## Boundaries de Code

Regles CI obligatoires:

- `scope:oss` ne depend jamais de `scope:cloud` ou `scope:internal`;
- `libs/*/products` ne depend pas de `adapters-*`;
- `apps/*` ne contient pas de logique metier persistante;
- aucun provider proprietaire n'apparait dans un crate/package exportable OSS;
- aucun fichier source touche ne depasse 500 lignes;
- aucun document prive n'est present dans l'export public;
- les SDKs sont regeneres depuis contracts, pas maintenus manuellement.

## Risques Bloquants

| Risque | Mitigation obligatoire |
|---|---|
| modele data legacy ambigu | inventaire row-level, reject log et decision owner |
| sessions incompatibles | invalidation explicite et UX de reconnexion |
| objets storage manquants | rapport bloqueur ou exclusion documentee |
| divergence billing | ledger reconcile et freeze des mutations billing |
| event replay non idempotent | consumer idempotency keys et DLQ testee |
| rollback lent | rehearsal rollback avec chronometrage |
| fuite Cloud/Internal dans OSS | allowlist export et checks boundaries bloquants |

## Ordre de Livraison

L'ordre privilegie reduit les dependances implicites:

1. platform primitives et contracts;
2. Identity;
3. Workspace/Authz;
4. Audit et Privacy;
5. Drive;
6. Billing et Usage;
7. Developer Platform;
8. Frontends produits;
9. Cloud Console et Internal Admin;
10. infra production, repetitions, cutover, decommission.

## Criteres de Reussite

La structuration est consideree valide quand:

- tous les parcours critiques ont une implementation nouvelle;
- tous les contrats publics et internes sont versionnes;
- toutes les donnees migrables sont reconciliees;
- aucun composant runtime legacy n'est requis;
- aucun provider proprietaire n'est dans le core OSS;
- aucun import interdit OSS/Cloud/Internal ne passe la CI;
- les backups et rollbacks sont testes;
- le repo public OSS ne contient aucun document prive;
- le runbook de cutover peut etre execute sans decision implicite;
- chaque gate a une preuve attachee: logs, rapports, checksums ou lien CI;
- chaque risque bloquant a ete ferme, accepte par owner, ou retire du scope.
