# Strategie Produit nvbes

## Definition

nvbes est une composition de produits numeriques dedies aux outils de travail, de stockage, de collaboration, de creation et d'automatisation.

Les produits peuvent avoir leurs propres offres, leurs propres cycles de maturite et leurs propres contraintes de marge. La plateforme commune doit fournir les primitives transverses: Identity, workspaces, billing, audit, consentement, quotas, privacy, developer APIs, observabilite et residence des donnees.

## Positionnement

Le positionnement cible est B2B pour tous.

Interpretation:

- l'experience reste accessible a une personne seule;
- chaque utilisateur recoit un workspace personnel dedie a la creation du compte;
- les workspaces personnels sont separes des workspaces de groupe;
- les offres personnelles servent d'entree de gamme rentable ou quasi rentable;
- les offres Team, Workspace et Business portent la marge et la gouvernance avancee;
- Business ne doit pas etre lance en production au demarrage, mais les contrats techniques doivent permettre de l'activer plus tard.

nvbes ne doit pas etre positionne comme du stockage brut moins cher. La valeur vendue est le controle professionnel, la confidentialite, la transparence, la residence EU/FR progressive et les workflows securises autour des fichiers et donnees.

## Invariants Produit

- Un `user` est global.
- Chaque nouveau `user` obtient automatiquement un workspace personnel.
- Le workspace personnel appartient au user et ne remplace jamais un workspace de groupe.
- Un workspace de groupe represente une equipe, une organisation, un client ou une activite partagee.
- Les droits, quotas, billing, policies, liens, transferts et audits sont scopes par workspace.
- Le switch entre workspaces doit respecter membership, niveau d'authentification, policy client, risque et device policy.

## Politique Donnees

Les donnees collectees par nvbes servent uniquement deux finalites:

- produire de l'Open Data, des statistiques publiques ou des jeux de donnees publics agregees et anonymisees;
- servir directement les produits internes nvbes et leur securite.

nvbes ne vend pas et ne vendra jamais les donnees personnelles des utilisateurs.

La politique produit est la transparence:

- l'utilisateur doit savoir quelles donnees sont collectees;
- l'utilisateur doit savoir pour quel usage;
- le consentement doit etre explicite quand l'usage n'est pas strictement necessaire au service;
- les analytics produit ne doivent pas contenir de contenu fichier, nom de fichier, token, secret, email en clair ou identifiant public direct.

## Surface Produit Cible

### Cloud

Cloud est le premier produit commercialise.

Capacites cibles:

- best-in-class sync technology;
- easy and secure sharing;
- anytime, anywhere access, d'abord EU puis extension progressive;
- unlimited devices;
- backup;
- account recovery and version history;
- restore deleted files;
- multi-factor authentication;
- document scanning;
- remote device wipe apres service dedie d'identification fiable des devices;
- watermarking;
- account transfer tool;
- granular permissions;
- ransomware detection and recovery;
- suspicious activity alerts;
- end-to-end encryption;
- shared links;
- advanced link settings;
- disable downloads;
- custom expiration dates;
- password-protected links;
- one-way transfer avec limite a definir par plan;
- incoming transfer requests;
- transfer analytics;
- password-protected transfers;
- eSignature;
- signature requests;
- advanced security and privacy;
- Drive for desktop;
- support for over 100 file types.

Le chiffrement end-to-end doit etre traite comme une capacite account-level pour workspace personnel ou workspace-level pour groupe. L'activation doit deleguer clairement la responsabilite au client et desactiver les features incompatibles.

### Produits Futurs

Produits a deployer apres stabilisation Cloud:

- Docs;
- Sheets;
- Slides;
- Forms;
- Photo Editor type Lightroom;
- Video Editor;
- Developer Hub avec acces aux APIs publiques.

Chaque produit peut avoir son propre pricing, mais doit reutiliser les primitives de consentement, workspace, billing, audit, quotas et privacy.

## Add-ons

Les add-ons doivent rester explicites et opt-in.

Familles possibles:

- stockage additionnel;
- retention avancee;
- egress ou transfert avance;
- signatures et enveloppes eSignature;
- scan documentaire/OCR;
- IA, vectorisation et entrainement LLM;
- securite avancee;
- support prioritaire;
- modules creation media.

## Sequence Production Initiale

Objectif: production a cout minimal, sans bloquer l'expansion EU.

Ordre cible:

1. Deployer uniquement Identity sur Scaleway avec l'architecture la moins couteuse et la plus optimisee possible, optimisee France uniquement.
2. Deployer Drive/Cloud full feature production sur la meme logique minimum cost.
3. Deployer Developer Hub avec acces aux APIs publiques.
4. Migrer vers une vraie base DevOps pour preparer l'elargissement EU.
5. Integrer ingestion de volume data, transformation, vectorisation et entrainement LLM.
6. Deployer Docs, Sheets, Slides puis Forms.
7. Deployer les plans Team et Workspace.
8. Preparer Business sans le vendre ni l'activer en production tant que les preuves produit, securite, support et marge ne sont pas etablies.

## Contraintes de Decision

- Ne pas lancer une offre qui ne peut pas etre tenue operationnellement.
- Ne pas rendre une feature disponible si elle contredit le modele E2EE choisi.
- Ne pas collecter une donnee sans finalite produit, securite, billing, support, legal ou Open Data explicite.
- Ne pas activer Business tant que SSO, SCIM, audit avance, support, legal, SLA, incidents et billing enterprise ne sont pas prets.
