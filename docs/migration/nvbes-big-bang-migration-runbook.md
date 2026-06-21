# Runbook Migration Big Bang nvbes

## Objectif

Decrire l'execution operationnelle de la migration complete vers la nouvelle
structure nvbes reconstruite depuis zero.

Ce runbook complete le [Plan Structuration Complete](../blueprint/nvbes-full-restructure-big-bang-zero-debt.plan.md).
Il sert a preparer, repeter, executer et verifier la bascule Big Bang.

## Principes d'Execution

- La nouvelle plateforme est construite en parallele.
- L'ancien systeme devient source de donnees pendant la migration.
- La bascule se fait en une fenetre controlee.
- La dette temporaire est interdite au cutover.
- Toute erreur data est corrigee avant la bascule ou explicitement rejetee.
- Toute etape critique est repetee avant production.
- Le rollback est teste avant la fenetre finale.

## Statut Operationnel - 2026-06-17

Etat actuel: no-go production.

Les artefacts de controle sont initialises et valides par les checks repository,
mais ils ne remplacent pas les decisions humaines ni les preuves de repetition.

Valide localement:

- `pnpm check`;
- `pnpm check:migration-inventory`;
- `pnpm check:migration-data-map`;
- `pnpm check:migration-secret-map`;
- `pnpm check:migration-job-map`;
- `pnpm check:migration-resource-map`;
- `pnpm check:migration-risk-register`;
- `pnpm check:migration-gate-evidence`;
- `pnpm check:migration-artifacts`.

No-go attendu:

- `pnpm check:migration-precutover -- --env production --reconciliation-report
  docs/migration/reconciliation.<run>.json` echoue tant que les decisions,
  preuves et reconciliation finale ne sont pas signees;
- `tools/migration/gate-evidence.mjs --strict` echoue tant que les gates G0 a
  G8 n'ont pas owner, preuves et decision `go`;
- les checks stricts de data map, secrets, jobs, ressources et risques doivent
  rester bloquants tant que les decisions ne sont pas signees;
- aucun cutover ne peut etre programme avec seulement les artefacts generes.

## Roles

| Role | Responsabilite |
|---|---|
| Migration lead | sequence globale, go/no-go, journal de cutover |
| Data lead | snapshots, export, transform, import, reconciliation |
| Product leads | validation par domaine et acceptation des rejects |
| Infra lead | deploy, DNS/edge, secrets, backup, rollback |
| Security lead | secrets, audit, acces, export OSS, incidents |
| Support lead | communication maintenance, statut client, post-cutover |

Une meme personne peut porter plusieurs roles hors production; en production, Migration lead et Data lead doivent etre separes.

## Artefacts Obligatoires

- `docs/migration/inventory.md`: inventaire source signe;
- `docs/migration/parity-matrix.md`: parite fonctionnelle par domaine;
- `docs/migration/owner-signoff-matrix.md`: owners et decisions signes;
- `docs/migration/data-map.md`: mapping table/champ/source/cible;
- `docs/migration/snapshot-manifest.md`, `release-freeze-manifest.md`, `backup-restore-manifest.md`: cible et restore;
- `docs/migration/reconciliation-report.md`: resultats par repetition;
- `docs/migration/rehearsal-ledger.md`: preuves des trois repetitions;
- `docs/migration/rejects.md`: donnees rejetees, decision et owner;
- `docs/migration/cutover-checklist.md`, `smoke-test-manifest.md`, `observability-readiness.md`, `live-evidence-instances/`, `live-evidence.template.json`: cutover;
- `docs/migration/communication-plan.md`: messages maintenance et rollback;
- `docs/migration/cutover-journal.md`: journal horodate des actions cutover;
- `docs/migration/gate-evidence.md`: preuves et decisions des gates G0 a G8;
- `docs/migration/rollback-report.md`: preuve rollback et duree mesuree;
- `docs/migration/risk-register.md`: risques bloquants, owner et mitigation;
- `docs/migration/post-migration-audit.md`, `decommission-manifest.md`, `v2-debt-register.md`: audit final.

## Commandes de Validation

Les commandes exactes peuvent etre adaptees au CI final, mais les categories
sont obligatoires:

```bash
pnpm migration:generate && pnpm verify
pnpm nx:affected
cargo check --workspace
pnpm check:web
pnpm lint:web
pnpm check:contracts && pnpm check:codegen
pnpm check:product-boundaries
pnpm check:migration-cli-options && pnpm check:migration-blueprint && pnpm check:migration-runbook && pnpm check:migration-tooling && pnpm check:migration-repository-controls
pnpm check:migration-inventory
pnpm check:migration-data-map && pnpm check:migration-data-migration-pipeline
pnpm check:migration-secret-map
pnpm check:migration-job-map
pnpm check:migration-resource-map && pnpm check:migration-target-structure && pnpm check:migration-codegen && pnpm check:migration-supply-chain && pnpm check:migration-runtime-foundation && pnpm check:migration-platform-primitives && pnpm check:migration-identity-register && pnpm check:migration-identity-login-session && pnpm check:migration-identity-mfa-webauthn && pnpm check:migration-workspace-membership-roles && pnpm check:migration-workspace-last-owner && pnpm check:migration-drive-upload-download && pnpm check:migration-drive-share-revoke && pnpm check:migration-drive-quotas && pnpm check:migration-audit-append-only && pnpm check:migration-privacy-export-delete && pnpm check:migration-billing-entitlements && pnpm check:migration-billing-webhook-idempotency && pnpm check:migration-developer-oauth-tokens && pnpm check:migration-developer-signed-webhooks && pnpm check:migration-cloud-provisioning && pnpm check:migration-frontend-experience && pnpm check:migration-infra-deploy && pnpm check:migration-phase-ledger && pnpm check:migration-domain-ledger && pnpm check:migration-domain-dod
pnpm check:migration-risk-register
pnpm check:migration-cutover-gates && pnpm check:migration-gate-evidence && pnpm check:migration-readiness-report && pnpm check:migration-completion-audit && pnpm check:migration-execution-backlog && pnpm check:migration-status-consistency && pnpm check:migration-cutover-evidence-packet && pnpm check:migration-live-evidence-schema && pnpm check:migration-live-evidence-rules && pnpm check:migration-live-evidence-instances && pnpm check:migration-live-evidence-commands && pnpm check:migration-live-evidence-prepare && pnpm check:migration-parity && pnpm check:migration-owner-signoffs && pnpm check:migration-cutover-checklist && pnpm check:migration-rehearsals && pnpm check:migration-snapshots && pnpm check:migration-release-freeze && pnpm check:migration-backup-restore && pnpm check:migration-communication && pnpm check:migration-observability && pnpm check:migration-smoke-tests && pnpm check:migration-rollback-report && pnpm check:migration-rejects && pnpm check:migration-reconciliation-report
pnpm check:migration-cutover-journal && pnpm check:migration-post-migration-audit && pnpm check:migration-decommission && pnpm check:migration-v2-debt && pnpm check:migration-reconciliation-template && pnpm check:migration-artifacts
pnpm check:migration-frontend-experience && pnpm check:web && pnpm check:migration-smoke-tests -- --strict && pnpm check:migration-infra-deploy && pnpm check:migration-backup-restore -- --strict && pnpm check:migration-rollback-report -- --strict && pnpm check:migration-rehearsals -- --strict && node tools/migration/reconcile.mjs --env staging --report docs/migration/reconciliation.<run>.json && pnpm check:migration-rejects -- --strict && pnpm check:migration-precutover -- --env production --reconciliation-report docs/migration/reconciliation.<run>.json && pnpm check:migration-postcutover && node tools/migration/reconcile.mjs --env production --report docs/migration/reconciliation.<run>.json
pnpm check:secrets && pnpm check:supply-chain
```

Pour la plateforme cible, ajouter les equivalents:
```bash
pnpm check:go && pnpm check:python
tools/migration/reconcile.mjs --env staging --report docs/migration/reconciliation-report.md
tools/migration/reconcile.mjs --env staging --report docs/migration/reconciliation.<run>.json
tools/migration/live-evidence-prepare.mjs --id <evidence-id> --type reconciliation --env production --owner "<owner>" --source-artifact docs/migration/reconciliation.<run>.json --command "node tools/migration/reconcile.mjs --env production --report docs/migration/reconciliation.<run>.json" --immutable-reference artifact://<immutable-run-artifact> --packet-requirement final-reconciliation --out docs/migration/live-evidence-instances/<evidence-id>.json
tools/oss-export/check
tools/security/scan-secrets
```

Une commande manquante est traitee comme no-go tant qu'elle n'a pas un substitut
documente et approuve par le Migration lead.

## Seuils Go/No-Go

| Zone | Go | No-go |
|---|---|---|
| Auth | login, refresh, MFA et logout passent | login global ou MFA critique KO |
| Data | counts/checksums attendus ou rejects signes | ecart non explique sur data critique |
| Drive | objets storage critiques presents | objets manquants sans decision owner |
| Billing | ledger et entitlements reconciles | double facturation ou entitlement perdu |
| Queues | lag sous seuil et DLQ vide ou acceptee | consumer non idempotent ou DLQ inconnue |
| Observabilite | logs, traces, metrics et alertes visibles | production aveugle |
| Rollback | rollback possible dans la fenetre annoncee | route de retour non testee |
| OSS export | aucun document prive ou provider interdit | fuite Cloud/Internal ou secret |

## Phase 0 - Freeze et Inventaire

Actions:

- geler les nouvelles features non critiques;
- inventorier endpoints, jobs, tables, events, buckets, secrets, docs et infra;
- classer chaque element: garder, reconstruire, supprimer, remplacer;
- etablir une matrice de parite fonctionnelle;
- definir la fenetre de coupure acceptable;
- definir les seuils go/no-go.

Livrables:

- `docs/migration/inventory.md`;
- `docs/migration/parity-matrix.md`;
- `docs/migration/data-map.md`;
- `docs/migration/cutover-checklist.md`;
- `docs/adr/0002-platform-clean-rebuild.md`.

Criteres de sortie:

- chaque fonctionnalite active a une cible nouvelle;
- chaque table source a une decision;
- chaque secret a un owner;
- chaque job critique a un equivalent;
- chaque risque bloquant a un plan de mitigation.

## Phase 1 - Fondation Parallele

Actions:

- creer la nouvelle arborescence monorepo;
- configurer les workspaces Rust, Go, TypeScript et Python;
- ajouter les checks de boundaries OSS/Cloud/Internal;
- ajouter CI: format, lint, typecheck, tests, cargo check, go test, pnpm checks;
- ajouter scans: secrets, licences, SBOM, dependances, containers;
- ajouter generation OpenAPI, Protobuf et schemas events;
- ajouter Docker, Helm, OpenTofu OSS et Cloud skeletons.

Criteres de sortie:

- un service minimal compile dans chaque runtime;
- les checks d'import interdits echouent correctement;
- la CI bloque secrets, licences incompatibles, contrats invalides et gros fichiers;
- staging peut etre cree from scratch.

## Phase 2 - Primitives Plateforme

Actions:

- construire config, errors, request ID et correlation ID;
- construire tenant context et region context;
- construire idempotency, audit writer et outbox writer;
- definir ports storage, email, queue, billing, search, analytics, kyc et kms;
- fournir au moins un adapter OSS par port requis;
- ajouter observability OpenTelemetry, metrics et logs structures;
- standardiser pagination et API error envelope.

Criteres de sortie:

- chaque primitive a tests unitaires;
- aucun produit ne depend d'un provider concret;
- chaque mutation critique peut emettre audit et outbox dans la meme transaction;
- chaque service expose health, readiness et metrics.

## Phase 3 - Migration Domaines

### Identity

Reconstruire user global, tenants, memberships, sessions, MFA, WebAuthn,
OAuth2/OIDC, service accounts, scaffold SAML/SCIM, risk et audit auth.

Migration data:

- exporter users, memberships, sessions et credentials compatibles;
- transformer vers le nouveau modele;
- importer en staging;
- reconcilier counts, checksums, orphelins et invariants;
- invalider proprement les sessions incompatibles.

Criteres: register, login, logout, refresh, MFA et OAuth passent en E2E;
aucune action sensible sans audit; aucun principal orphelin apres import.

### Workspace et Authz

Reconstruire tenants, organizations, workspaces, roles, policies, invitations et
decisions authz.

Criteres: owner, admin, member et viewer testes; last-owner protection active;
permission refusee auditee; modele workspace/authz partage par les produits.

### Drive

Reconstruire metadata fichiers/dossiers, upload sessions, signed URLs,
download, trash, restore, delete, share links, quotas, public API v1, privacy et
malware scan port.

Migration data:

- migrer metadata;
- verifier existence des objets storage;
- recalculer quotas;
- produire rapport objets manquants;
- reconstruire audit minimal si necessaire.

Criteres: upload/download/share/revoke/delete/restore passent en E2E; aucun
objet public permanent; liens expires; quotas alignes sur l'usage storage.

### Billing et Usage

Reconstruire plans, entitlements, usage ledger, metering, invoice abstraction,
webhooks idempotents, events billing et FinOps cost attribution.

Adapters:

- OSS: adapter local ou Lago;
- Cloud: Stripe.

Criteres: aucun workspace payant sans entitlement; aucun entitlement sans source
billing; webhooks rejouables sans double effet; ledger reconcile.

### Developer Platform

Reconstruire portal developer, OAuth app management, webhooks publics signes,
token inspector, docs API, SDK TypeScript/Rust/Go et exemples d'integration.

Criteres: SDKs generes depuis contrats, exemples compiles, webhooks signes et
rejouables.

## Phase 4 - Frontends

Actions:

- reconstruire Identity Web, Drive Web, Developer Web, Cloud Console et Internal Admin;
- brancher uniquement les SDKs generes ou clients types;
- appliquer design system et WCAG AA.

Criteres: aucun fetch non type, aucun `any`, parcours Playwright critiques,
budgets bundles et screenshots desktop/mobile valides.

## Phase 5 - Infra et Operations

Actions:

- produire images OCI immuables;
- signer images avec Cosign/Sigstore;
- generer SBOM;
- deployer Helm/OpenTofu en staging;
- configurer secrets manager;
- configurer backups chiffres;
- tester restore;
- configurer dashboards et alerting;
- ecrire runbooks incidents;
- tester rollback.

Criteres: staging reconstruit from scratch, production EU recreable, restore
backup valide et rollback valide avant cutover.

## Phase 6 - Repetitions Migration

Executer au minimum trois repetitions:

1. dry run local sur snapshot;
2. dry run staging avec donnees anonymisees;
3. dress rehearsal sur snapshot production.

Chaque repetition produit:

- duree export, transformation et import;
- erreurs et donnees rejetees;
- checksums et ecarts par domaine;
- actions manuelles;
- decision go/no-go.

Criteres de sortie:

- 0 erreur bloquante;
- duree compatible avec la fenetre de coupure;
- rollback teste;
- reconciliation stable entre deux repetitions consecutives.

Format de rapport par repetition:

| Champ | Contenu attendu |
|---|---|
| snapshot_id | identifiant immutable du snapshot source |
| version cible | commit, images OCI, migrations SQL et contracts |
| durees | export, transform, import, reconciliation, smoke |
| data | counts, checksums, orphelins, duplicats, rejects |
| incidents | cause, impact, correction, owner |
| decision | go, no-go, ou go avec rejects signes |

Une repetition est invalide si elle utilise une procedure manuelle non reportee
dans la checklist de cutover.

## Phase 7 - Cutover Big Bang

Sequence:

1. annoncer maintenance;
2. passer l'ancien systeme en read-only;
3. stopper les workers anciens;
4. prendre snapshot final;
5. exporter les donnees finales;
6. transformer les donnees;
7. importer dans la nouvelle plateforme;
8. executer reconciliation finale;
9. basculer DNS/edge vers la nouvelle stack;
10. activer les workers nouveaux;
11. lancer smoke tests;
12. surveiller SLO;
13. valider go/no-go final.

Smoke tests obligatoires:

- login;
- MFA;
- workspace access;
- upload/download;
- share link;
- billing entitlement;
- webhook replay;
- privacy export;
- audit event;
- worker queue.

Go final si:

- smoke tests critiques passent;
- reconciliation finale sans ecart bloquant;
- erreurs API et queue lag sous seuil;
- logs et metrics visibles;
- rollback encore possible.

### Fenetre Type

| Temps | Action | Owner |
|---|---|---|
| T-7j | annoncer maintenance et freeze non critique | Support lead |
| T-48h | confirmer dernier rehearsal, rollback et seuils | Migration lead |
| T-24h | verifier backups, capacity, secrets et images | Infra lead |
| T-2h | geler deploys et confirmer equipe presente | Migration lead |
| T-0 | activer maintenance, read-only et stop workers legacy | Infra lead |
| T+15m | snapshot final et export | Data lead |
| T+60m | transform/import et rapport rejects | Data lead |
| T+90m | reconciliation finale | Data lead |
| T+105m | bascule DNS/edge et demarrage workers nouveaux | Infra lead |
| T+120m | smoke tests et go/no-go final | Product leads |
| T+180m | communication retour service ou rollback | Support lead |

Les temps sont des budgets cibles. Le runbook final doit remplacer ces valeurs
par les durees observees pendant les repetitions.

### Checklist Cutover Minimale

- commit cible, images OCI et migrations SQL figes;
- exports legacy et snapshots marques read-only;
- secrets production presents dans la nouvelle stack;
- DNS/edge rollback documente;
- dashboards ouverts pour API, DB, queues, workers et edge;
- smoke tests automatises prets;
- canal incident ouvert;
- decision makers presents;
- freeze billing confirme;
- procedure de rollback chronometree disponible.

## Rollback

Rollback immediat si:

- corruption de donnees detectee;
- login global impossible;
- upload/download critique impossible;
- billing entitlement massivement incorrect;
- reconciliation finale echoue;
- observabilite production indisponible.

Actions rollback:

1. stopper nouvelle stack publique;
2. desactiver workers nouveaux;
3. remettre DNS/edge vers ancien systeme;
4. restaurer ancien systeme en read-only ou read-write selon risque;
5. conserver snapshots et logs de cutover;
6. ouvrir incident interne;
7. analyser avant nouvelle tentative.

Un rollback partiel par feature flag est autorise seulement si aucune dette
temporaire n'est introduite et si les donnees restent coherentes.

## Reconciliation

La reconciliation est bloquante pour les donnees critiques:

- row counts par table source/cible;
- checksums par domaine et par tenant quand possible;
- detection des orphelins: users, memberships, files, billing accounts;
- verification object storage: metadata cible vers objet existant;
- quotas recalcules et compares a l'usage storage;
- ledger billing balance et idempotency keys;
- replay outbox sur environnement non public;
- absence de duplicats sur cles metier.

Chaque ecart doit etre classe:

- `fixed`: corrige avant cutover;
- `accepted`: accepte par owner avec justification;
- `rejected`: donnees exclues avec preuve et notification si necessaire;
- `blocking`: no-go.

## Communication

Avant cutover:

- annoncer la fenetre, l'impact utilisateur et le point de statut;
- preparer message maintenance et page statut;
- preparer message rollback si la fenetre echoue.

Pendant cutover:

- journaliser horodatage, action, owner et resultat;
- communiquer uniquement les statuts valides par Migration lead;
- ne pas annoncer de succes avant smoke tests et reconciliation.

Apres cutover:

- publier retour service;
- maintenir surveillance renforcee;
- ouvrir canal support dedie pendant la periode d'hypercare.

## Phase 8 - Decommission

Actions:

- garder l'ancien systeme archive en read-only pendant la verification;
- supprimer chemins runtime legacy, scripts, configs et secrets inutilises;
- archiver exports de migration;
- mettre a jour docs OSS, Cloud et Internal;
- produire audit post-migration;
- ouvrir backlog V2 seulement apres validation zero dette.

Criteres de fin:

- aucun service, endpoint, job ou secret legacy actif;
- aucun import boundary violation;
- aucun document prive expose dans OSS;
- rapport post-migration approuve.

## Validation Globale

- Structure: boundaries, file size, import graph, licences, secrets, SBOM, containers.
- Contrats: OpenAPI, SDK generation, Protobuf, event schemas, webhooks.
- Metier: Identity, Workspace/Authz, Drive, Billing, Privacy, Developer Platform, Audit.
- Data: row counts, checksums, object storage, quotas, orphelins, duplicats, replay.
- Operations: backup, restore, rollback, dashboards, alerting, incident channel.
- Security: secret rotation, least privilege, audit append-only, export OSS propre.
- Support: annonces, page statut, hypercare, rapport post-migration.

Validation finale:

- `post-migration-audit.md` approuve par Migration lead, Security lead et owners
  produit;
- tous les secrets legacy inutiles sont revoques;
- tous les jobs legacy sont arretes ou supprimes;
- tous les runbooks publics et prives sont a jour;
- aucun backlog V2 ne contient une dette necessaire au bon fonctionnement V1.
