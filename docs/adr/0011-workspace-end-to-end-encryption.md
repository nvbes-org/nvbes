# ADR 0011 - Adopter un chiffrement end-to-end optionnel par workspace

## Status

Proposed.

## Date

2026-08-02

## Context

Cloud chiffre deja certaines donnees au repos et pendant leur transport. Ces
protections limitent l'exposition aux operateurs d'infrastructure et aux
interceptions reseau, mais elles ne constituent pas un chiffrement end-to-end
si un service nvbes peut obtenir la cle de dechiffrement.

Le produit souhaite proposer des workspaces dans lesquels nvbes, ses
operateurs et ses sous-traitants ne peuvent pas lire les fichiers. Cette
propriete modifie profondement la gestion des cles, le partage, la recuperation
de compte et les fonctionnalites qui executent un traitement serveur sur le
contenu.

Un mode partiel active fichier par fichier rendrait les garanties difficiles a
comprendre et permettrait a des metadonnees ou contenus sensibles de sortir
involontairement de la frontiere chiffree. Un protocole different par client
rendrait egalement les fichiers incompatibles entre Web, desktop, mobile et TV.

Les mecanismes existants nommes `request E2EE` protegent des payloads
applicatifs avec une configuration connue du serveur. Ils ne fournissent pas la
propriete zero-knowledge attendue pour les fichiers Cloud et ne doivent pas
etre presentes comme telle.

## Decision

Cloud propose deux modes de workspace explicitement distincts :

- le mode standard, dans lequel nvbes assure le chiffrement au repos avec des
  cles gerees cote serveur et peut fournir les traitements serveur autorises ;
- le mode end-to-end, dans lequel les cles permettant de dechiffrer le contenu
  sont generees, distribuees et conservees exclusivement par les clients
  autorises.

Le mode est une propriete du workspace entier. Il n'est pas selectionne fichier
par fichier. Le changement de mode exige une migration complete et verifiable
de toutes les donnees concernees ; une simple bascule de configuration est
interdite.

### Threat model

Le mode end-to-end protege la confidentialite et l'integrite des contenus face
a une compromission du stockage, de la base de metadonnees, de l'infrastructure
Cloud ou d'un operateur serveur ne possedant pas les cles clientes.

Il ne protege pas un contenu deja dechiffre sur un appareil compromis. Il ne
peut pas reprendre une cle ou un fichier deja copie par un ancien membre. La
resistance a un serveur actif qui substitue les cles publiques d'appareils
exige une verification des appareils et, avant toute promesse correspondante,
un mecanisme audite de transparence des cles.

### Key hierarchy

Chaque appareil genere localement une identite cryptographique distincte. Les
cles privees ne quittent l'appareil que dans un coffre de recuperation chiffre
de bout en bout.

Chaque workspace end-to-end possede une cle racine versionnee par generation.
Chaque fichier et chaque version de fichier recoivent une cle de contenu
aleatoire et independante. Les cles de contenu sont enveloppees par la
generation active de la cle du workspace. La compromission d'une cle de
contenu ne doit pas exposer les autres objets.

La cle de workspace est distribuee aux appareils autorises au moyen d'enveloppes
chiffrees avec HPKE, selon la RFC 9180, ou d'une construction standardisee
offrant des garanties equivalentes apres revue cryptographique. Les cles ne
sont jamais transmises en clair au serveur.

L'ajout d'un membre ou appareil cree une nouvelle enveloppe autorisee. Le
retrait d'un membre cree une nouvelle generation de cle pour les futurs
contenus et versions. Revoquer l'acces cryptographique a des contenus anciens
requiert leur rechiffrement avec de nouvelles cles de contenu ; re-envelopper
seulement une ancienne cle ne suffit pas si le membre a pu la conserver.

### File encryption format

Les fichiers sont chiffres par flux avec une primitive d'encryption
authentifiee adaptee aux contenus volumineux. La cible initiale est
`crypto_secretstream_xchacha20poly1305` de libsodium, sous reserve de validation
de sa disponibilite et de son comportement sur chaque shell supporte.

Le format chiffre est versionne et independant du runtime de presentation. Il
authentifie au minimum la version du protocole, le workspace, l'objet, la
version du fichier et l'ordre des blocs. Les clients doivent partager des
vecteurs de test communs et refuser les versions, suites ou transitions non
supportees.

Les implementations utilisent une bibliotheque cryptographique auditee. Elles
ne composent pas directement des primitives Web Crypto dans un nouveau
protocole propre a chaque plateforme.

### Recovery and device enrollment

Le mot de passe nvbes n'est jamais la cle racine du workspace. Sa modification
ne provoque pas le rechiffrement des fichiers.

Le premier client genere une cle de recuperation a haute entropie et demande a
l'utilisateur d'en conserver une copie hors ligne. Le serveur peut conserver
un coffre chiffre, mais ne possede pas le secret permettant de l'ouvrir. Un
nouvel appareil est autorise par un appareil existant ou par cette cle de
recuperation.

Une phrase saisie par l'utilisateur ne peut proteger un coffre qu'apres
derivation avec Argon2id et des parametres mesures sur la plateforme cible. Une
phrase faible ne remplace pas une cle de recuperation aleatoire.

Une organisation Enterprise peut choisir une cle de recuperation
administrateur. Cette capacite est optionnelle, auditee et affichee clairement
aux membres. Un workspace avec recuperation administrateur ne peut pas etre
presente comme inaccessible a l'organisation.

### Metadata and server capabilities

Les noms de fichiers, contenus, apercus, miniatures et autres metadonnees
sensibles sont chiffres lorsque le serveur n'en a pas besoin pour router,
stocker ou appliquer une limite explicite. Le serveur conserve seulement les
identifiants opaques, relations structurelles minimales, tailles chiffrees,
timestamps techniques et donnees necessaires aux quotas, a la retention et a
l'audit sans contenu.

Les fonctions exigeant le contenu en clair sont executees sur un client
autorise ou sont indisponibles. Cela concerne notamment :

- la recherche plein texte et l'indexation serveur ;
- les apercus, miniatures, OCR et transformations media serveur ;
- l'antivirus et la classification de contenu cote serveur ;
- la deduplication fondee sur le contenu ;
- la recuperation de contenu par le support ;
- les traitements IA effectues sur les fichiers par nvbes.

L'interface expose ces incompatibilites avant l'activation. Aucune
fonctionnalite ne peut contourner silencieusement le mode end-to-end en envoyant
une version en clair au serveur.

### Sharing

Le partage interne utilise les identites cryptographiques des appareils
autorises. Un lien public end-to-end place le secret de dechiffrement dans le
fragment de l'URL ou dans un canal client equivalent qui n'est pas transmis au
serveur. Le jeton d'acces serveur et le secret de dechiffrement sont distincts.

Un lien protege par une phrase choisie par l'utilisateur doit resister aux
attaques hors ligne ; les secrets aleatoires generes par le client restent la
preference. Les limites d'expiration, revocation et nombre de telechargements
continuent d'etre appliquees par Cloud sans lui donner acces au contenu.

### Delivery sequence

Le deploiement suit cet ordre :

1. figer le threat model, le format versionne et les vecteurs de test ;
2. implementer les workspaces personnels sur Web et desktop ;
3. valider la recuperation et l'enrolement de plusieurs appareils ;
4. faire auditer independamment le protocole et ses implementations ;
5. ajouter les workspaces de groupe et la rotation de membership ;
6. ajouter mobile puis TV avec un appairage securise lorsque necessaire.

Le mode n'est pas annonce comme disponible en production avant la revue
cryptographique independante et la validation des parcours de perte de cle.

## Consequences

- Une compromission serveur ou stockage ne suffit plus a lire les fichiers des
  workspaces end-to-end.
- Les utilisateurs obtiennent une frontiere de confidentialite explicite et
  verifiable, differente du chiffrement au repos standard.
- La perte de tous les appareils et de la cle de recuperation rend les donnees
  definitivement inaccessibles.
- La gestion multi-appareil, le partage de groupe, la rotation et les migrations
  de format deviennent des responsabilites clientes critiques.
- Certaines fonctionnalites Cloud sont indisponibles ou plus couteuses parce
  qu'elles doivent etre executees localement.
- Les clients Web ont une frontiere de confiance plus faible qu'un binaire
  signe si le serveur qui distribue le JavaScript est activement compromis ; la
  supply chain, la signature des artefacts et la transparence des cles font
  partie du modele de securite.
- Le support ne peut pas recuperer silencieusement un contenu end-to-end et doit
  expliquer clairement cette limite.
- Le format et les contrats cryptographiques deviennent des interfaces de
  compatibilite durables entre toutes les plateformes.

## Non-goals

Cette decision ne choisit pas un chiffrement homomorphe, ne promet pas de
revoquer des donnees deja telechargees et ne rend pas anonymes les metadonnees
reseau ou de facturation. Elle ne remplace pas TLS, le chiffrement au repos, la
securite des appareils ou les controles d'autorisation Cloud.

MLS n'est pas introduit pour le stockage V1. Il pourra etre evalue separement
si les groupes massifs ou la collaboration temps reel exigent ses proprietes de
forward secrecy et de post-compromise security.

## Validation

La decision est correctement appliquee uniquement si :

- aucun service nvbes ne peut reconstruire les cles de contenu end-to-end ;
- chaque fichier et version utilisent une cle de contenu independante ;
- le format chiffre est versionne et couvert par des vecteurs multi-plateformes ;
- les ajouts, retraits, rotations et conflits de generation sont testes ;
- la perte et la restauration d'appareils sont testees sans acces serveur aux
  secrets ;
- un workspace end-to-end ne transmet jamais silencieusement un contenu en
  clair a une fonctionnalite serveur ;
- les limitations produit sont visibles avant activation ;
- le protocole et au moins une implementation de reference ont recu une revue
  cryptographique independante avant la production.
