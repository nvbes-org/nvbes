# Drive Web Redesign Design

Date: 2026-06-08

## Goal

Refondre entierement `apps/cloud-web` pour obtenir une V1 front credible de nvbes Drive: une vraie application de travail pour petites equipes, pas une page de demonstration.

La refonte couvre l'ensemble du front Drive V1 visible: fichiers, liens partages, corbeille, membres, securite, facturation, API et compte. Elle reste une refonte frontend: les endpoints existants sont utilises quand ils sont disponibles, et les domaines manquants sont representes par des donnees locales realistes avec interactions front completes.

## Product Direction

Le produit cible reste celui du PRD Drive V1:

- drive europeen securise pour petites equipes;
- usage quotidien B2B, dense, calme et efficace;
- activation rapide par upload, partage controle et invitation membre;
- securite visible mais non intrusive;
- pas de derive vers une suite collaborative complete.

La barre de qualite demandee est `prototype produit credible`. L'application doit convaincre visuellement et couvrir les workflows principaux, sans exiger que tout le backend V1 soit deja disponible.

## Visual Direction

Drive reprend le theme de `account-web`:

- typographie `Geist`;
- surfaces claires;
- blanc froid, graphite, gris brume;
- accent petrole ou vert profond;
- bordures fines;
- radius modere;
- composants shadcn locaux;
- densite proche des pages compte Identity.

La personnalite Drive combine:

- `outil premium calme`;
- `console securite legere`.

L'identite visuelle doit venir de la precision de l'outil: tables bien dessinees, statuts lisibles, metadonnees utiles, mono pour les identifiants et evenements techniques, badges sobres et actions claires. Il ne faut pas introduire de hero, de decoration marketing, de gradients dominants, d'orbes, ni de patterns visuels qui contredisent les guidelines produit existantes.

## UX Architecture

L'application utilise une navigation a deux niveaux.

Le premier niveau est un rail compact permanent sur desktop. Il sert a choisir les grands modules:

- Drive;
- Partage;
- Administration;
- Compte.

Le second niveau depend du module actif:

- Drive: Fichiers, Corbeille;
- Partage: Liens partages, Activite partage si utile;
- Administration: Membres, Securite, Facturation, API;
- Compte: Profil, sessions ou preferences selon les donnees disponibles.

La zone centrale affiche la vue de travail. Un panneau contextuel droit apparait seulement quand il aide a prendre une decision ou a agir: fichier selectionne, lien actif, membre, facture, cle API ou evenement sensible.

Sur mobile, le rail devient une navigation compacte ouvrable et le panneau contextuel devient un sheet. La priorite est `desktop-first, mobile utilisable`: consultation, recherche, partage rapide, copie de lien, suivi d'upload et revocation critique doivent rester possibles sur mobile.

## Core Layout

Le layout desktop contient:

- rail de module compact;
- sous-navigation contextuelle;
- header compact;
- zone de contenu principale;
- panneau de details contextuel optionnel.

Le header affiche:

- breadcrumb;
- recherche globale;
- statut reseau ou upload;
- quota;
- menu compte.

Les actions de vue restent dans une toolbar locale:

- importer;
- nouveau dossier;
- bascule table/grille;
- filtre;
- tri;
- actions groupees quand une selection existe.

## Primary View: Files

La vue `Fichiers` est la reference pour toute l'application. Elle definit la densite, les interactions, les etats et la grammaire visuelle des autres vues.

Elle doit inclure:

- table dense par defaut;
- bascule grille pour dossiers et medias;
- drag-and-drop upload;
- dialog ou zone d'upload avec progression;
- breadcrumb;
- recherche;
- filtres;
- tri;
- selection multiple;
- actions groupees;
- menu par ligne;
- panneau de details contextuel.

Le panneau de details est cache par defaut. Il s'ouvre quand un fichier ou dossier est selectionne, ou via une action `Details`.

Le panneau affiche:

- metadonnees;
- statut partage;
- activite recente;
- statut securite;
- proprietaire;
- taille;
- date de modification;
- actions principales.

## V1 Views

### Shared Links

La vue `Liens partages` sert de centre de controle des liens publics:

- fichier ou dossier;
- createur;
- permission;
- expiration;
- nombre d'acces ou telechargements;
- statut actif, expire ou revoque;
- copier le lien;
- modifier l'expiration;
- revoquer.

Aucun lien public ne doit etre presente comme permanent. L'expiration obligatoire reste un principe visible de la V1.

### Trash

La vue `Corbeille` couvre:

- liste des fichiers et dossiers supprimes;
- date de suppression;
- expiration de conservation;
- restauration;
- suppression definitive;
- etat vide.

Les suppressions definitives doivent passer par une confirmation.

### Members

La vue `Membres` couvre:

- membres actuels;
- roles;
- statut invitation;
- derniere activite;
- invitation locale realiste si l'API manque;
- changement de role mocke si necessaire.

Les roles visibles restent les roles produit V1: proprietaire, admin, membre, lecteur.

### Security

La vue `Securite` regroupe:

- liens publics actifs;
- actions sensibles recentes;
- politiques de partage;
- signaux de session ou acces si disponibles;
- export du journal d'activite si disponible.

Elle doit renforcer la confiance sans devenir l'ecran principal de l'app.

### Billing

La vue `Facturation` affiche:

- plan actuel;
- usage stockage;
- utilisateurs inclus et utilises;
- prochaine facture estimee;
- usage additionnel si visible;
- alertes quota;
- moyen de paiement;
- factures;
- actions upgrade, downgrade ou portail client selon les capacites disponibles.

### API

La vue `API` couvre:

- cles API;
- prefixe visible;
- scopes;
- statut;
- expiration;
- derniere utilisation;
- createur;
- copie de prefixe ou identifiant;
- revocation.

Le copy de securite doit rester direct: les cles API donnent acces aux fichiers de l'espace et doivent etre protegees comme des mots de passe.

### Account

La vue `Compte` doit rester coherente avec `account-web`. Elle peut renvoyer vers Identity quand une fonctionnalite appartient deja a l'identite globale, au lieu de dupliquer une interface incomplete dans Drive.

## Data Strategy

La refonte utilise une strategie hybride:

- brancher les donnees et operations deja disponibles;
- mocker localement les domaines incomplets;
- exposer une interface de donnees stable pour pouvoir remplacer progressivement les mocks par des appels API;
- produire les memes etats UX pour donnees reelles et mockees.

Les interactions front mockees doivent etre completes:

- pending;
- success;
- error;
- toast ou feedback inline;
- confirmation pour les actions destructives;
- mutation locale visible immediatement quand c'est coherent.

Les donnees front doivent etre typees explicitement. Le type `any` est interdit.

## Component Architecture

La refonte reste dans `apps/cloud-web` et suit les conventions du repo:

- fichiers courts;
- noms explicites;
- pas de fichier fourre-tout;
- pas de barrel massif;
- composants d'assemblage minces;
- logique de donnees et logique UI separees.

Composants structurants prevus:

- `DriveAppLayout`;
- `DriveModuleRail`;
- `DriveSectionNav`;
- `DriveCommandHeader`;
- `DriveFilesView`;
- `DriveFilesTable`;
- `DriveFilesGrid`;
- `DriveDetailsPanel`;
- `DriveActionToolbar`;
- `DriveEmptyState`;
- `DriveErrorState`;
- `DriveLoadingState`;
- `DriveSharedLinksView`;
- `DriveTrashView`;
- `DriveMembersView`;
- `DriveSecurityView`;
- `DriveBillingView`;
- `DriveApiKeysView`;
- `DriveAccountView`.

Les noms finaux peuvent etre ajustes pendant le plan d'implementation si le code existant impose une meilleure frontiere, mais les responsabilites doivent rester aussi nettes.

## Critical Flows

Les flux critiques a couvrir sont:

- premiere arrivee dans un espace vide;
- upload de fichiers;
- creation de dossier;
- recherche, filtre et tri;
- selection multiple;
- ouverture du panneau de details;
- creation, copie et revocation de lien;
- restauration depuis corbeille;
- invitation membre;
- changement de role;
- lecture des signaux securite;
- consultation quota et facturation;
- revocation de cle API.

## Error And State Design

Chaque vue critique doit couvrir:

- chargement;
- vide;
- succes;
- erreur;
- desactivation;
- acces refuse;
- temporisation ou API indisponible quand pertinent.

Les messages doivent etre actionnables:

- upload echoue: proposer `Reessayer` ou `Retirer`;
- quota atteint: expliquer la limite et envoyer vers facturation;
- permission refusee: expliquer le role requis;
- lien expire ou revoque: afficher un statut clair;
- API indisponible: garder l'app consultable si des donnees locales existent, avec un bandeau discret.

Les etats ne doivent jamais dependre uniquement de la couleur.

## Motion

La motion est limitee au feedback utile:

- ouverture et fermeture du panneau de details;
- apparition des lignes apres chargement;
- feedback de selection;
- progression upload;
- confirmation de copie ou revocation;
- transitions courtes des sheets et dialogs.

Les animations doivent rester courtes, interruptibles et compatibles avec `prefers-reduced-motion`.

## Accessibility

La refonte doit respecter:

- labels visibles pour les champs;
- focus clavier visible;
- actions icon-only avec labels accessibles;
- contrastes lisibles;
- support clavier des actions principales;
- messages d'etat perceptibles sans couleur seule;
- tailles de cibles tactiles acceptables sur mobile.

## Testing And Validation

Les tests cibles doivent couvrir prioritairement:

- rendu de la vue fichiers;
- bascule table/grille;
- recherche;
- selection;
- actions mockees;
- panneau details;
- confirmation destructive;
- empty states;
- error states.

La validation attendue apres implementation frontend est:

- check cible `cloud-web` si disponible;
- sinon `pnpm check:web`;
- lint web si le scope touche beaucoup de fichiers.

## Scope Boundaries

Inclus:

- refonte frontend complete de `apps/cloud-web`;
- theme aligne sur `account-web`;
- vues V1 front;
- donnees hybrides;
- interactions locales realistes;
- etats critiques;
- architecture composant propre.

Exclus:

- refonte backend Drive;
- edition collaborative;
- sync desktop;
- apps mobiles natives;
- permissions fines par fichier ou dossier;
- preview publique riche;
- nouveau design system partage hors scope;
- site marketing ou prototype navigateur separe.

