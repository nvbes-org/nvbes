# Analyse Concurrentielle - nvbes Drive

> **Statut : recherche produit future.** Cloud/Drive n'est pas le premier
> produit présumé et cette analyse ne fixe aucune priorité V1. Elle devra être
> actualisée après sélection du produit. Voir la
> [direction produit](nvbes-product-strategy.md).

Etat initial au 2026-06-06.

## Objectif

Comprendre les alternatives que les petites equipes comparent mentalement avant d'adopter nvbes Drive.

L'analyse ne vise pas a copier les concurrents. Elle sert a identifier:

- les categories d'achat;
- les promesses dominantes;
- les prix de reference;
- les objections probables;
- les espaces de positionnement defendables.

## Categories Concurrentielles

### Suites Generalistes

Acteurs:

- Google Workspace / Google Drive;
- Microsoft 365 / OneDrive;
- Dropbox Business.

Lecture:

- ce sont les options par defaut;
- elles beneficient du status quo;
- elles sont percues comme fiables et completes;
- elles rendent difficile une bataille sur le prix ou les fonctionnalites.

```text
nvbes ne doit pas se presenter comme un Google Drive moins cher. Le terrain plus defendable est le controle des partages clients pour petites equipes.
```

### Acteurs Europeens ou Privacy-First

Acteurs:

- kDrive / Infomaniak;
- Leviia Drive;
- Proton Drive;
- Tresorit;
- pCloud;
- Internxt.

Lecture:

- ils capturent les acheteurs sensibles a la confidentialite, a la souverainete et au RGPD;
- ils installent des preuves attendues: hebergement local, chiffrement, certifications, controle des liens;
- ils montrent que le marche accepte des alternatives non americaines.

```text
Le message "europeen" est necessaire mais insuffisant. Il doit etre lie a un usage concret: liens clients visibles, expiration, revocation, audit.
```

### Transfert Ponctuel

Acteurs:

- WeTransfer;
- Smash;
- liens temporaires generes par les outils existants.

Lecture:

- ils resolvent le besoin rapide d'envoyer un fichier;
- ils ne structurent pas un espace d'equipe durable;
- ils peuvent etre suffisants pour les freelances ou petites equipes sans processus.

```text
nvbes doit prouver que le probleme n'est pas seulement "envoyer un fichier", mais "savoir ce qui reste partage apres l'envoi".
```

### Solutions Internes ou Techniques

Acteurs:

- NAS;
- serveur interne;
- Nextcloud auto-heberge;
- object storage brut.

Lecture:

- ils attirent les profils techniques;
- ils promettent le controle, mais ajoutent maintenance, securite et support;
- ils sont souvent trop lourds pour des equipes de 2 a 10 personnes.

```text
nvbes peut se positionner comme le controle sans maintenance interne.
```

## Matrice Concurrentielle Initiale

| Alternative | Promesse dominante | Prix de reference observe | Force | Faiblesse exploitable |
| --- | --- | --- | --- | --- |
| Google Workspace | Suite collaborative complete | Starter a 7 USD/user/mois, Standard a 14 USD/user/mois | Habitude, suite complete, confiance | Peu differenciant sur controle simple des liens clients pour petites equipes |
| Microsoft 365 | Suite bureautique et IT complete | Business Basic a 6 USD/user/mois annuel, 1 To/user | Standard entreprise, email, Office | Complexite, ecosysteme large, achat IT plus que workflow client |
| Dropbox Business | Sync, stockage et partage equipe | Standard a 15 USD/user/mois, minimum 3 utilisateurs | Marque fichier forte, sync, partage | Positionnement large, prix par siege, moins "europeen" |
| kDrive | Cloud suisse competitif et collaboratif | Team a 10 CHF/mois, Pro a 6.66 CHF/user/mois | Prix agressif, stockage, OnlyOffice, Suisse | Offre large; message controle client moins specialise |
| Leviia Drive | Cloud souverain francais pour pros et institutions | Pro a 8 EUR HT/user/mois, a partir de 10 utilisateurs | France, HDS/ISO27001, suite collaborative | Minimum 10 utilisateurs; cible plus institutionnelle |
| Proton Drive | Confidentialite et chiffrement | Drive Professional a 7.99 EUR/user/mois selon pages regionales | Marque privacy forte, chiffrement, Suisse/Allemagne | Peut etre percu comme privacy suite plus que drive d'equipe client |
| Tresorit | Partage securise et controle avance | File Sharing Business autour de 14.50 USD/user/mois selon index G2; prix officiel parfois masque dynamiquement | Tres fort sur securite, liens, logs, zero-knowledge | Plus enterprise/securite; prix et richesse peuvent depasser la petite equipe |
| WeTransfer / Smash | Envoi rapide de gros fichiers | Freemium / plans pro | Simplicite, usage ponctuel | Pas un espace d'equipe de controle continu |
| NAS / serveur interne | Controle complet | Cout variable + maintenance | Controle, propriete | Charge operationnelle, securite, acces externe |

## Signaux Concurrentiels Observes

### Google Workspace

Google vend une suite complete. La page pricing consultable le 2026-06-06 affiche notamment:

- Starter a 7 USD/user/mois apres promotion;
- Standard a 14 USD/user/mois apres promotion;
- 30 Go par utilisateur sur Starter;
- 2 To par utilisateur sur Standard.

Source: [Google Workspace pricing](https://workspace.google.com/pricing.html)

Lecture marketing:

- Google est le choix par defaut, pas l'ennemi frontal.
- La faiblesse n'est pas la capacite produit, mais la perception d'un outil generaliste ou les liens clients peuvent rester disperses.

### Dropbox Business

Dropbox affiche:

- Standard a 15 USD/user/mois;
- minimum 3 utilisateurs;
- 5 To de stockage equipe;
- controle admin, roles, partage, restauration 180 jours.

Source: [Dropbox Business plans](https://www.dropbox.com/business/plans-comparison)

Lecture marketing:

- Dropbox est fort quand la douleur principale est "sync et fichiers".
- nvbes doit eviter la comparaison stockage/sync et parler de controle client, expiration et visibilite.

### Microsoft 365 / OneDrive

Microsoft 365 Business Basic est presente a 6 USD/user/mois annuel avec 1 To de stockage cloud par utilisateur.

Source: [Microsoft 365 plans](https://www.microsoft.com/en-us/microsoft-365/business/microsoft-365-plans-and-pricing)

Lecture marketing:

- Microsoft est difficile a battre sur valeur bundlee.
- Les petites equipes non IT peuvent cependant percevoir l'ecosysteme comme large et administratif.

### kDrive / Infomaniak

kDrive affiche:

- Team a 10 CHF/mois;
- Pro a 6.66 CHF/user/mois;
- Team avec 6 utilisateurs inclus;
- Pro a partir de 3 utilisateurs;
- stockage a partir de 3 To ou 6 To selon offre;
- OnlyOffice et, sur Pro, Microsoft Office Online.

Source: [kDrive prices](https://www.infomaniak.com/en/ksuite/kdrive/prices)

Lecture marketing:

- kDrive pose un prix europeen/suisse tres agressif.
- nvbes ne doit pas se battre au volume de stockage.
- L'espace possible est la specialisation "petites equipes qui partagent des fichiers clients avec controle".

### Leviia Drive

Leviia affiche:

- Pro a 8 EUR HT/mois/utilisateur;
- depart a 10 utilisateurs;
- certifications ISO27001 et HDS mises en avant;
- repartition sur 3 zones en France;
- gestion granulaire des utilisateurs, groupes et quotas.

Source: [Leviia Drive pricing](https://www.leviia.com/en/pricing-leviia-drive/)

Lecture marketing:

- Leviia est fort sur souverainete francaise, institutions et offre collaborative.
- Le minimum 10 utilisateurs laisse un espace pour les equipes de 2 a 10, si l'offre nvbes reste simple.

### Proton Drive

Proton met en avant:

- chiffrement de bout en bout;
- metadata protegee;
- stockage sur serveurs en Suisse ou Allemagne;
- ISO 27001;
- conformite GDPR et HIPAA;
- essai 14 jours sur les offres business.

Sources:

- [Proton Drive Business pricing](https://proton.me/business/drive/pricing)
- [Proton Drive for Business](https://proton.me/business/drive)

Lecture marketing:

- Proton gagne sur confiance privacy et marque.
- nvbes doit etre plus concret sur le workflow fichier client, plutot que plus ambitieux sur la privacy generale.

### Tresorit

Tresorit met en avant:

- liens securises et chiffres;
- expiration, mot de passe, limite d'ouverture;
- notifications et logs;
- personnalisation de liens;
- stockage chiffre;
- ISO 27001, GDPR, zero-knowledge;
- offres business et enterprise.

Source: [Tresorit File Sharing pricing](https://tresorit.com/m/pricing/file-sharing)

Lecture marketing:

- Tresorit est le concurrent le plus proche sur "partage securise".
- nvbes doit eviter de promettre plus de securite que Tresorit en V1.
- L'angle defendable est plus localise: petites equipes francophones/europeennes, simplicite, controle des liens clients, offre accessible.

## Opportunites de Positionnement

### Opportunite 1 - Controle des Liens Clients

```text
Tous vos liens clients visibles, expirables et revocables.
```

- concret;
- lie a une peur claire;
- different d'une promesse generique de stockage;
- facile a tester en entretien.

### Opportunite 2 - Drive de Travail Client

```text
Un espace propre pour stocker et partager les fichiers clients sensibles.
```

- parle aux agences et cabinets;
- ne promet pas de remplacer toute la suite bureautique;
- deplace la comparaison vers le professionnalisme client.

### Opportunite 3 - Europeen Sans Complexite Enterprise

```text
Le controle europeen des fichiers, sans lourdeur enterprise.
```

- capte le besoin de confiance;
- evite la promesse "souverainete" trop abstraite;
- parle aux equipes trop petites pour les offres institutionnelles.

## Objections Attendues

Pourquoi ne pas rester sur Google Drive ?

- Reponse a tester: parce que le probleme n'est pas seulement de stocker, mais de savoir quels liens clients restent actifs.

Pourquoi changer si Dropbox fonctionne ?

- Reponse a tester: Dropbox est excellent pour synchroniser; nvbes doit etre meilleur pour controler les partages clients d'une petite equipe.

Pourquoi payer alors que WeTransfer suffit ?

- Reponse a tester: WeTransfer envoie; nvbes garde une memoire d'equipe sur ce qui a ete partage, par qui, et jusqu'a quand.

Pourquoi faire confiance a un nouvel acteur ?

- Reponse a tester: transparence, hebergement europeen, politiques simples, beta fermee, preuves visibles, support fondateur.

## Questions d'Investigation Concurrentielle

- Quel outil serait remplace ou complete par nvbes ?
- Le prospect compare-t-il nvbes a Google Drive, Dropbox, WeTransfer ou a un NAS ?
- Le probleme percu est-il stockage, transfert, securite, souverainete ou controle ?
- Quelle fonctionnalite concurrente est non negociable ?
- Quelle fonctionnalite concurrente est visible mais peu utilisee ?
- Quelle preuve ferait passer de "interessant" a "j'essaie" ?

## Prochaine Etape

- captures de pricing;
- entretiens utilisateurs;
- scoring par segment;
- table detaillee des fonctionnalites de partage;
- messages concurrents exacts;
- avis clients et irritants recurrents.
