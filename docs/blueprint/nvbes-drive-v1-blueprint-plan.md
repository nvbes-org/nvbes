# Blueprint V1 + Plan d'Execution - nvbes Drive

## Statut

**Remplacé comme plan V1 actif.** Ce blueprint est conservé comme hypothèse de
produit future. Cloud/Drive ne fait pas partie du socle V1 et ne doit pas être
implémenté à partir de ce plan sans sélection explicite du premier produit. Voir
la [direction produit canonique](../product/nvbes-product-strategy.md).

## Objectif

Hypothèse historique : livrer une future version vendable de nvbes Drive pour
petites équipes de 2 à 10 personnes :

- hebergement EU-first;
- partage de fichiers simple et controle;
- securite visible mais non intrusive;
- billing et quotas fiables;
- fondations reutilisables pour les futurs micro-SaaS nvbes.

## Positionnement

```text
Le drive europeen securise pour les petites equipes.
```

## Resultat Produit Attendu

Un utilisateur doit pouvoir:

1. creer un compte;
2. creer un espace;
3. uploader un premier fichier en moins de 10 minutes;
4. generer un lien partage avec expiration obligatoire;
5. inviter un membre;
6. comprendre son quota, son essai et son plan;
7. passer en payant sans rupture de service.

## Scope V1 Cadre

Inclus:

- auth;
- workspaces personnels et equipe;
- membres et roles;
- fichiers et dossiers;
- upload/download via signed URLs;
- corbeille;
- liens de partage publics avec expiration et revocation;
- quotas;
- audit basique;
- billing par abonnement;
- base d'usage metered;
- export/suppression privacy;
- API publique V1 limitee aux integrations fichiers.

Exclus:

- permissions par dossier/fichier;
- sync desktop;
- apps mobiles natives;
- edition collaborative;
- versioning avance;
- OCR, recherche IA;
- SSO/SAML;
- preview publique riche.

## Principes de Conception

- `Tenant` porte la gouvernance securite et identity; `workspace` reste le contexte produit principal.
- Les fichiers vivent dans l'object storage; PostgreSQL ne stocke que les metadonnees.
- Aucun objet public permanent en V1: URLs signees courtes, tokens de partage hashes, buckets prives.
- Le backend V1 est un backend modulaire unique structure par domaine, pas des microservices.
- La V1 privilegie la fiabilite des parcours critiques sur la largeur fonctionnelle.
- Le billing et la conformite ne sont pas une couche finale: ils font partie du coeur produit.

## Blueprint Fonctionnel

### 1. Auth et Identite

Capacites:

- inscription, verification email, login, logout;
- reset password securise;
- user global avec memberships `tenant`, `organization` optionnelle et `workspace`;
- gestion des sessions globales et du contexte workspace;
- tokens hybrides: JWT courts + introspection/decision centrale sur actions sensibles;
- step-up enterprise tier et MFA;
- clients OAuth approuves ou interdits par tenant/workspace;
- invalidation de sessions apres changement de mot de passe ou mutation de role sensible.

Exigences:

- verification email obligatoire avant usage complet;
- rate limiting sur login, register et reset password;
- journalisation des succes et echecs de login;
- MFA obligatoire pour comptes internes/admin nvbes;
- l'email n'est jamais l'identifiant fort du systeme;
- les endpoints critiques doivent pouvoir exiger `authz/decision` et `step-up`.

### 2. Workspaces et Roles

Modele:

- `personal` pour solo;
- `team` pour petites equipes.

Roles V1:

- `owner`
- `admin`
- `member`
- `viewer`

Regles:

- permissions produit V1 principalement au niveau workspace sur un socle identity multi-tenant;
- policies de partage administrees par owner;
- les actions sensibles sont auditees.

### 3. Drive Core

Capacites:

- liste fichiers/dossiers;
- creation dossier;
- renommage;
- deplacement;
- corbeille;
- restauration;
- suppression definitive;
- recherche simple.

UX cible:

- vue liste/table dense;
- sidebar stable;
- breadcrumb;
- toolbar contextuelle;
- quota toujours visible.

### 4. Upload et Download

Upload flow:

1. creation d'une upload session;
2. verification auth, permission, plan et quota;
3. creation d'un `StorageObject` `pending`;
4. retour d'une signed upload URL courte;
5. upload direct object storage;
6. completion avec taille/checksum;
7. verification backend;
8. activation objet;
9. mise a jour quota + audit.

Contraintes:

- object keys opaques;
- upload session usage unique et courte duree;
- revalidation quota pour eviter les contournements concurrents;
- purge des uploads expires par worker.

Download flow:

- generation de signed download URL courte apres verification des permissions;
- audit des acces de telechargement.

### 5. Partage Public Controle

Capacites:

- creation de lien;
- expiration obligatoire;
- revocation immediate;
- limite de telechargement optionnelle;
- journalisation des acces publics.

Contraintes:

- aucun lien public sans expiration;
- TTL par defaut: 7 jours;
- TTL max par plan;
- rate limiting dedie aux routes publiques;
- pas d'exposition des object keys;
- pas de preview publique riche en V1;
- scan ou quarantaine anti-malware avant exposition publique.

### 6. Quotas et Usage

Capacites:

- stockage utilise;
- nombre de fichiers;
- bande passante sortante du mois;
- alertes 80% et 100%;
- blocage de nouveaux uploads apres depassement critique et grace period.

Mesures V1:

- `storage_gb_month`
- `team_seat_month`
- `egress_gb` en suivi

### 7. Billing et FinOps

Capacites:

- essai 14 jours sans CB;
- upgrade plan;
- checkout Stripe;
- customer portal Stripe;
- estimation de facture;
- affichage usage et quota;
- ledger interne des usages;
- traitement webhook idempotent.

Regles de gouvernance:

- Stripe source de verite pour paiement et facture;
- nvbes source de verite pour droits produit, quotas, usages et audit;
- aucun usage additionnel facture sans visibilite produit prealable;
- revue obligatoire de toute modification prix/quota.

### 8. Audit, Privacy et Compliance

Capacites:

- audit events consultables par owner/admin;
- export audit;
- export de donnees utilisateur et workspace;
- suppression compte et workspace;
- retention documentee;
- runbooks incident et procedures RGPD.

Exigences:

- audit append-only au niveau applicatif;
- journaliser `permission.denied`, acces partages publics, telechargements, changements billing;
- restauration backup compatible avec les suppressions RGPD deja traitees.

### 9. API Publique V1

Capacites:

- API keys legacy par workspace; creation desactivee, list/revocation seulement;
- scopes;
- rotation/revocation;
- endpoints REST `/v1` limites aux operations fichiers;
- OpenAPI versionnee.

Contraintes:

- cles hashees en base;
- affichage une seule fois;
- rate limits par plan;
- audit des actions API sensibles;
- pas de breaking change dans `/v1`.

## Blueprint Technique

### Stack

- monorepo;
- backend Rust;
- frontend TypeScript + React + Vite;
- `Effect` cote frontend quand utile pour la fiabilite des workflows;
- Cloudflare en edge;
- Scaleway pour compute, PostgreSQL, object storage et backups.

### Bounded Contexts Backend

- auth;
- workspaces;
- members;
- files;
- uploads;
- downloads;
- share-links;
- quotas;
- billing;
- audit;
- privacy;
- public-api.

### Composants Runtime

- application web frontend;
- API backend modulaire unique;
- PostgreSQL;
- object storage S3-compatible;
- workers asynchrones;
- Stripe;
- provider email;
- pipeline anti-malware/quarantaine.

### Environnements

- `development`
- `staging`
- `production`

Chaque environnement a sa DB, ses buckets, ses secrets, ses webhooks et ses logs.

### Workers V1

- purge corbeille;
- nettoyage uploads expires;
- recalcul quotas;
- snapshots usage;
- webhooks billing;
- exports RGPD;
- suppression compte/workspace;
- nettoyage liens expires;
- emails transactionnels.

### Observabilite

Minimum V1:

- logs structures JSON;
- `request_id`/`correlation_id`;
- dashboards API, uploads/downloads, workers, PostgreSQL, object storage, billing et security;
- alertes sur 5xx, upload failures, jobs bloques, webhooks billing, backup failure, budget cloud.

## Blueprint UX

Direction:

- `B2B sobre et dense`

Principes:

- interface calme, lisible, peu decorative;
- securite visible mais discrete;
- pas de look SaaS generique;
- densite moyenne-haute;
- motion minimale et utile.

Navigation V1:

- Fichiers
- Liens partages
- Corbeille
- Membres
- Facturation
- Securite
- API
- Compte

Priorites UX:

- premier upload < 2 minutes;
- pas de choix de plan avant premiere activation;
- etats critiques soignes: quota, upload echoue, permission refusee, lien expire/revoque, billing echoue, workspace suspendu.

## Blueprint de Release

Definition of Done V1:

- parcours critiques E2E passes;
- compliance et docs legales en place;
- pricing et TVA valides;
- observabilite et alerting actifs;
- backups testes;
- runbooks incidents rediges;
- OpenAPI publique publiee;
- marge brute minimale estimee >= 50% au lancement.

## Livrables Identity a Figer

- architecture identity cible;
- modele de donnees cible;
- contrats de contexte et de token;
- roadmap identity enterprise;
- backlog/scaffold federation et SCIM.

## Plan d'Execution Etape par Etape

### Phase 0. Cadrage de lancement

1. Valider officiellement le scope V1 et geler les hors-scope.
2. Nommer les owners produit, tech, design, security/privacy et billing.
3. Transformer les docs existantes en backlog epics + milestones.
4. Valider les hypotheses business critiques: pricing, trial, quotas, marge cible, ICP.
5. Figer les conventions techniques du monorepo, des migrations et des environnements.

### Phase 1. Fondations repo et plateforme

1. Initialiser le monorepo frontend/backend/infrastructure.
2. Creer les bases frontend React/Vite/TypeScript et backend Rust.
3. Installer le design system de base (`shadcn/ui`, `Radix UI`, `Tailwind CSS`).
4. Mettre en place CI/CD minimal avec lint, tests, builds et scans.
5. Provisionner `development` et `staging` en IaC.
6. Configurer Cloudflare, Scaleway PostgreSQL et object storage prives.
7. Mettre en place secret management, logs structures et monitoring de base.

### Phase 2. Schema de donnees et contrats

1. Implementer le schema PostgreSQL pour `User`, `Session`, `Workspace`, `WorkspaceMember`, `StorageObject`, `UploadSession`, `ShareLink`, `QuotaUsage`, `AuditEvent`, `Billing*`, `ApiKey`.
2. Ecrire les migrations versionnees et les seeds minimaux.
3. Formaliser les DTO et erreurs standard de l'API.
4. Poser les invariants metier critiques: roles, statuts, append-only audit, hash tokens, object keys opaques.
5. Rediger ou generer la base OpenAPI interne.

### Phase 3. Modele identity de reference et migrations conceptuelles

1. Figer le modele `user global -> tenant -> organization? -> workspace`.
2. Poser les concepts `user_identities`, memberships multi-scope et roles scopes.
3. Ecrire les migrations conceptuelles et les invariants d'heritage de policies.
4. Documenter les cas solo, petite equipe, org optionnelle et multi-tenant.
5. Poser les contrats `AuthContext`, `role_assignments` et `workspace access decision`.

### Phase 4. Sessions Redis, tokens hybrides et bootstrap produit

1. Implementer register, verify-email, login, logout et forgot/reset password.
2. Implementer la session globale, les refresh tokens rotatifs Redis et les tokens contextualises.
3. Introduire token global minimal pour `/me` et `/workspaces`.
4. Creer le flow de creation de workspace a l'inscription.
5. Initialiser trial 14 jours et quota trial 5 Go.
6. Auditer tous les evenements auth critiques.

### Phase 5. Memberships tenant/org/workspace, roles et switch workspace

1. Implementer les memberships `tenant`, `organization`, `workspace`.
2. Implementer les roles `owner/admin/member/viewer` au niveau workspace.
3. Implementer invitations membres et acceptation.
4. Implementer mutation de role et retrait membre.
5. Implementer `WorkspacePolicy` pour le partage membre.
6. Implementer la decision d'acces workspace selon membership, client policy et niveau d'auth.
7. Invalider les sessions Redis ou grants apres changement de role sensible ou suppression membre.

### Phase 6. MFA, step-up et WebAuthn

1. Implementer `auth_factors` et `auth_challenges` (Redis).
2. Implementer MFA et step-up pour actions sensibles.
3. Brancher WebAuthn/passkeys comme facteur fort cible.
4. Reserver les endpoints `oauth/introspect` et `authz/decision`.
5. Poser le scaffold devices/risk policy si non completement implemente.

### Phase 7. Scaffold federation et SCIM

1. Introduire `tenant_domains`, `federated_identity_providers` et `scim_provisioning_connectors`.
2. Documenter les interfaces reservees sans promettre leur implementation immediate.
3. Garantir que le modele identity ne bloque ni SAML/OIDC inbound ni provisioning futur.

### Phase 8. Drive core

1. Implementer dossiers et listing objets.
2. Implementer rename, move, trash, restore, delete.
3. Implementer recherche simple.
4. Construire la vue Fichiers dense avec etats vide/erreur/upload.
5. Ajouter audit et controle permission sur chaque action.

### Phase 9. Upload/download fiables

1. Implementer creation d'upload session.
2. Generer les signed upload URLs courtes.
3. Implementer completion d'upload avec verification taille/checksum.
4. Activer les objets et mettre a jour le quota.
5. Implementer download URLs signees.
6. Construire les workers de purge uploads expires et recalcul quota.
7. Ajouter les tests de concurrence quota et de reprise apres echec.

### Phase 10. Partage public securise

1. Implementer creation, listing, update et revocation de share links.
2. Hasher les tokens et interdire tout lien sans expiration.
3. Construire les routes publiques `/public/shares/:token`.
4. Ajouter rate limiting dedie.
5. Journaliser acces et telechargements.
6. Integrer le workflow anti-malware/quarantaine pour exposition publique.
7. Concevoir la vue Liens partages et les etats lien expire/revoque.

### Phase 11. Quotas, plans et experience billing

1. Implementer `Plan`, `Subscription`, `BillingAccount`, `UsageEvent`, `UsageSnapshot`, `InvoiceEstimate`.
2. Configurer les plans Solo Pro, Team, Team Plus.
3. Brancher Stripe Checkout et Customer Portal.
4. Implementer webhooks verifies, journalises et idempotents.
5. Projeter les statuts Stripe en droits produit internes.
6. Afficher quota, utilisateurs inclus/utilises, estimation de facture et alertes.
7. Bloquer les nouveaux uploads sur depassement critique apres grace period.

### Phase 12. Audit, privacy et administration securite

1. Implementer journal d'activite consultable par owner/admin.
2. Implementer export audit.
3. Implementer export utilisateur et export workspace.
4. Implementer suppression compte et suppression workspace via jobs.
5. Documenter les delais de suppression et la politique backup.
6. Construire la vue Securite avec liens publics actifs, sessions et actions sensibles.

### Phase 13. API publique V1

1. Implementer creation/revocation/rotation des cles API.
2. Implementer authentification par cle, scopes et rate limits.
3. Exposer les endpoints `/v1` du scope V1.
4. Ajouter audit API et `ApiRequestLog`.
5. Publier l'OpenAPI versionnee, guides auth/upload/download et exemples curl.
6. Ajouter l'ecran API dans le produit.

### Phase 14. Observabilite, exploitation et hardening

1. Finaliser dashboards et alertes critiques.
2. Mettre en place budgets cloud et suivi couts par environnement/workspace.
3. Tester backups et restoration.
4. Valider runbooks incidents API, PostgreSQL, storage, billing, RGPD.
5. Verifier qu'aucun bucket public n'est expose.
6. Passer en revue redaction des logs, secrets et analytics RGPD.

### Phase 15. Qualite, beta et lancement

1. Completer la suite unit/integration/E2E/smoke selon la strategie.
2. Executer la matrice de regression sur staging.
3. Realiser 3 a 5 tests utilisateurs cibles.
4. Corriger les points de friction sur activation et partage.
5. Verifier pricing, TVA, documents legaux et sous-traitants.
6. Lancer une beta fermee.
7. Suivre activation, conversion, cout par workspace et incidents.
8. Ouvrir le lancement payant si les seuils de fiabilite, marge et activation sont tenus.

## Ordre de Priorite Reel

Priorite 1:

- auth;
- modele identity de reference;
- sessions hybrides Redis;
- memberships;
- workspace;
- roles;
- drive core;
- upload/download;
- partage public;
- quotas;
- audit minimum.

Priorite 2:

- billing Stripe;
- MFA/step-up/WebAuthn;
- scaffold federation/SCIM;
- privacy jobs;
- observabilite complete;
- beta readiness.

Priorite 3:

- API publique V1;
- raffinement UX avance;
- optimisation FinOps.

## Jalons Recommandes

- Jalon A: fondations techniques + identity de reference + auth.
- Jalon B: drive utilisable avec upload/download.
- Jalon C: partage public securise + quotas + audit.
- Jalon D: billing + plans + experience facture.
- Jalon E: privacy + hardening + observabilite.
- Jalon F: API publique + beta + lancement payant.

## Risques de Delivery a Surveiller

- sous-estimer la complexite upload/quota/concurrence;
- traiter billing trop tard et decouvrir une marge negative;
- ne pas isoler assez tot audit, analytics et logs techniques;
- lancer les liens publics sans anti-malware ni rate limiting dedie;
- retarder les runbooks, backups et restore tests jusqu'a la fin;
- ouvrir l'API publique avant la maturite des scopes, logs et rate limits.

## Recommandation d'Execution

Pour une V1 robuste, il faut construire en tranches verticales:

1. auth + workspace + UI de base;
2. memberships + switch workspace + step-up;
3. objets + upload/download + quota;
4. partage public + audit + securite;
5. billing + plans + estimations;
6. privacy + API publique + hardening.

Cette sequence suit les risques reels du produit: architecture identity d'abord, contexte d'acces ensuite, fiabilite du stockage juste apres, monétisation ensuite, ouverture externe seulement une fois les garde-fous en place.
