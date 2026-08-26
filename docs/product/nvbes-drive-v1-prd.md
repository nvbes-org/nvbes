# PRD V1 - nvbes Drive

## Statut

**Hypothèse de produit future — hors V1 active.** Ce PRD est conservé pour une
éventuelle sélection de Cloud/Drive après validation du socle. Il ne doit pas
déclencher de travaux, beta, pricing ou infrastructure sans décision produit
séparée conforme à la
[direction V1 canonique](nvbes-product-strategy.md).

## Resume

Ce document explorait nvbes Drive, renommé fonctionnellement nvbes Cloud, comme
un cloud européen sécurisé. Ce positionnement n'est pas celui de la V1 du socle.

Le produit permet a une personne seule ou a un groupe de stocker, organiser, partager et controler ses fichiers dans un environnement EU-first, simple et fiable.

## Positionnement

```text
Le cloud europeen securise pour travailler seul ou en groupe avec des fichiers professionnels.
```

## Direction Experience

La direction produit et interface est `B2B sobre et dense`.

Les decisions de direction artistique, UI, UX, design system, accessibilite et copywriting sont definies dans [Design, UX et Design System](../product/design-ux.md).

## Cibles

Ancienne hypothèse de positionnement Drive :

- professionnel accessible en solo, avec espace personnel créé automatiquement
  pour chaque utilisateur ;
- Workspaces de groupe separes pour equipes, organisations et futurs plans Workspace.

Segments prioritaires:

- Independants professionnels.
- Agences et studios de 2 a 10 personnes qui partagent des fichiers clients.
- Cabinets de conseil.
- Associations structurees qui manipulent des documents sensibles.

Les details marketing, ICP, funnel, KPIs et analytics sont definis dans [Marketing, Conversion et Product Analytics](../product/marketing-growth.md).

## Promesse

```text
Stockez, organisez et partagez vos fichiers professionnels dans un cloud europeen simple et securise.
```

## Objectifs

- Permettre a chaque user de demarrer dans un workspace personnel dedie.
- Permettre a une equipe de gerer ses fichiers dans un workspace de groupe separe.
- Rendre le partage externe simple et controlable.
- Donner une visibilite claire sur l'usage, les liens actifs et la facturation.
- Poser les modules reutilisables de la plateforme nvbes pour les futurs micro-SaaS.
- Valider un modele economique rentable avec marge brute cible.
- Mesurer les couts variables par workspace avant d'elargir les quotas.
- Livrer une experience sobre, dense, accessible et orientee activation.

## Non-Objectifs

- Remplacer Google Workspace.
- Faire de l'edition collaborative de documents.
- Livrer un client desktop de synchronisation.
- Livrer des apps mobiles natives.
- Proposer du chiffrement end-to-end complet.
- Proposer de l'OCR ou de la recherche IA.
- Gerer des modeles de permissions enterprise complexes.
- Supporter SSO/SAML en V1.
- Lancer Business en production initiale.

## Scope V1

- Authentification.
- Creation automatique du workspace personnel.
- Workspaces de groupe separes.
- Upload et download de fichiers.
- Dossiers.
- Recherche simple.
- Renommer, deplacer, mettre en corbeille, restaurer et supprimer.
- Liens de partage securises.
- Expiration et revocation des liens.
- Membres et roles.
- Quotas par workspace.
- Facturation par abonnement.
- Fondation pour facturation a l'usage.
- Audit basique.
- Export et suppression RGPD.
- Chiffrement au repos.
- Controle des liens publics: expiration obligatoire, revocation, audit d'acces.
- Protection auth minimale: verification email, reset password securise, brute force protection.
- Acces admin interne protege par MFA, moindre privilege et journalisation.
- Liens publics V1 centres sur le telechargement controle, sans preview publique riche.

## Hors Scope V1

- Permissions par dossier ou fichier.
- Synchronisation desktop.
- Apps mobiles natives.
- Edition de documents type suite office.
- Chiffrement end-to-end complet.
- Versioning avance.
- OCR.
- Recherche IA.
- SSO enterprise.
- Plan Business en production.
- Docs, Sheets, Slides, Forms.
- Photo Editor et Video Editor.
- Ingestion data, vectorisation et entrainement LLM.

## Metriques de Succes

North Star V1:

```text
Nombre de workspaces actifs qui partagent au moins un fichier avec controle de lien sur 30 jours.
```

- Activation: pourcentage des nouveaux utilisateurs qui uploadent un fichier en moins de 10 minutes.
- Adoption groupe: pourcentage des comptes actifs qui creent ou rejoignent un workspace de groupe.
- Partage: nombre de liens de partage crees par workspace actif.
- Conversion: pourcentage des essais qui deviennent payants.
- Usage: stockage moyen utilise par workspace actif.
- Controle securite: nombre de liens actifs, expires et revoques.
- Securite auth: nombre de login failed, comptes MFA actifs, sessions revoquees.
- Hygiene partage: pourcentage de liens publics avec expiration courte.

Targets V1:

- Activation: 60% des nouveaux workspaces uploadent un fichier en moins de 10 minutes.
- Conversion trial to paid: 10% minimum au lancement, cible 20%.
- Marge brute: 50% minimum au lancement, cible 70%.
- ARPA personnel et groupe suivis separement.
- Churn mensuel logo: cible sous 5% apres les premiers clients.
- MRR premiere validation: 1 000 EUR.
- Cout infra par workspace actif: suivi hebdomadaire.
- Cout par To stocke: suivi mensuel.

Cadence:

- Revue hebdomadaire pendant beta.
- Revue mensuelle apres lancement payant.

## Risques

- Marge faible si le produit est vendu comme du stockage brut.
- Derive de scope vers un remplacement complet de Google Drive.
- Support couteux si l'upload/download n'est pas parfaitement fiable.
- Confiance difficile a gagner sans preuves visibles de securite et de confidentialite.
- Offre personnelle trop peu margee si les quotas et l'egress ne sont pas controles.
- Business active trop tot avant support, SLA, SSO/SCIM, audit avance et legal pack.
