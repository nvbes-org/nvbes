# Pricing

## Strategie

nvbes vise un positionnement B2B pour tous: utilisable seul, achetable par une petite structure, extensible vers equipe et workspace gouverne.

La strategie produit globale est definie dans [Strategie Produit nvbes](nvbes-product-strategy.md).

nvbes Cloud ne doit pas etre vendu comme du stockage brut moins cher. Le produit vend:

- workspace personnel separe des espaces de groupe;
- hebergement europeen avec depart France;
- securite et confidentialite;
- controle des liens, transferts et permissions;
- facturation lisible;
- API publique et automatisation;
- transparence sur les donnees collectees.

Les exigences FinOps, Stripe, TVA et unit economics sont definies dans [FinOps et Billing](finops-billing.md).

## Architecture d'Offres

Chaque utilisateur recoit un workspace personnel a la creation du compte. Ce workspace peut avoir une offre personnelle independante des workspaces de groupe.

Les workspaces de groupe sont separes:

- `team`: collaboration simple;
- `workspace`: gouvernance et controle avance;
- `business`: enterprise-ready, prepare mais non lance en production initiale.

## Plans Personnels

Role de conversion:

- proposer un prix minimum avec le moins de marge possible sans plan structurellement negatif;
- permettre l'usage professionnel solo;
- servir d'entree vers les offres plus rentables;
- limiter les abus par quotas, egress caps et trial controle.

Les prix exacts ne doivent pas etre figes avant le modele de cout Scaleway, Object Storage, egress, backups, logs, support et PSP. La regle est: aucun plan personnel public sans marge normale, scenario heavy user et seuil de blocage documentes.

### Personal Essential

Objectif: prix minimum.

Inclus:

- workspace personnel;
- stockage cloud securise;
- upload/download;
- dossiers;
- liens partages avec expiration;
- restauration des fichiers supprimes;
- MFA;
- sauvegarde et recovery de compte;
- acces multi-device;
- API publique basique limitee.

Contraintes:

- quotas stricts;
- egress limite;
- retention courte;
- support asynchrone;
- pas de gouvernance de groupe.

### Personal Plus

Objectif: premiere offre rentable.

Inclus:

- tout Personal Essential;
- plus de stockage;
- version history et recovery etendus;
- advanced link settings;
- liens proteges par mot de passe;
- disable downloads;
- expiration custom;
- document scanning apres disponibilite;
- transfer requests entrants.

### Personal Pro

Objectif: offre personnelle rentable pour independants et creators professionnels.

Inclus:

- tout Personal Plus;
- watermarking;
- one-way transfer avec limite a definir;
- transfer analytics;
- eSignature et signature requests selon disponibilite;
- support prioritaire leger;
- Drive for desktop apres disponibilite.

### Personal Secure

Objectif: offre personnelle premium securite.

Inclus:

- tout Personal Pro;
- suspicious activity alerts;
- ransomware detection and recovery;
- remote device wipe apres service dedie d'identification fiable des devices;
- end-to-end encryption account-level si compatible avec les features activees;
- advanced security and privacy.

## Plans Groupe

Les plans groupe ne sont pas la premiere mise en production payante obligatoire. Ils doivent etre prepares techniquement par le modele `tenant -> organization? -> workspace`.

### Team

Objectif: petites equipes qui partagent des fichiers clients.

Inclus:

- workspace de groupe;
- membres et roles;
- invitations;
- quotas groupe;
- audit basique;
- policies de partage;
- API publique V1 complete;
- service accounts OAuth pour integrations machine.

### Workspace

Objectif: organisations avec plusieurs espaces, controle accru et meilleure marge.

Inclus:

- tout Team;
- granular permissions;
- account transfer tool;
- reporting usage et transfert;
- limites API plus hautes;
- retention avancee;
- support prioritaire.

### Business

Statut: prepare mais non lance en production initiale.

Business exige avant activation:

- SSO/SAML;
- SCIM;
- audit logs avances;
- admin security reviews;
- SLA;
- runbooks incident client;
- legal pack B2B complet;
- support operationnel;
- validation marge et cout de support.

## Cloud - Capacites Cibles

Capacites a ranger par plan, add-on ou roadmap:

- best-in-class sync technology;
- easy and secure sharing;
- anytime, anywhere access, d'abord EU;
- unlimited devices;
- backup;
- account recovery and version history;
- restore deleted files;
- MFA;
- document scanning;
- remote device wipe apres service device fiable;
- watermarking;
- account transfer tool;
- granular permissions;
- ransomware detection and recovery;
- suspicious activity alerts;
- end-to-end encryption account-level ou workspace-level;
- shared links;
- advanced link settings;
- disable downloads;
- custom expiration dates;
- password-protected links;
- one-way transfer avec limite a definir;
- incoming transfer requests;
- transfer analytics;
- password-protected transfers;
- eSignature;
- signature requests;
- Drive for desktop;
- support for over 100 file types.

## Add-ons

Add-ons possibles:

- stockage additionnel;
- retention et version history etendues;
- egress/transfert additionnel;
- eSignature packs;
- OCR/document scanning;
- securite avancee;
- IA, vectorisation et entrainement LLM;
- support prioritaire;
- modules Docs, Sheets, Slides, Forms, Photo Editor et Video Editor quand disponibles.

## Trial

Essai recommande au lancement:

```text
14 jours
Sans carte bancaire
Quota faible
Workspace personnel cree automatiquement
Upgrade obligatoire pour debloquer les limites completes du plan
```

Protections anti-abus:

- verification email obligatoire;
- limite de workspaces par email, domaine et IP;
- friction ou blocage sur domaines email jetables;
- limite d'uploads et d'egress pendant trial;
- monitoring des trials couteux;
- suppression ou archivage des trials expires selon politique de retention.

## Principe Billing

L'usage facture doit etre visible avant d'etre facture.

Stripe est le provider billing cible pour la V1, avec ledger interne nvbes canonique.

La page facturation doit afficher:

- plan actuel;
- workspace personnel ou groupe facture;
- stockage inclus;
- stockage utilise;
- utilisateurs inclus pour les plans groupe;
- utilisateurs actifs;
- prochaine facture estimee;
- usage additionnel;
- add-ons actifs.

## TVA et Facturation EU

- Les prix publics doivent preciser HT ou TTC.
- Le billing doit distinguer B2B et B2C, meme si le positionnement marketing est B2B pour tous.
- Le numero de TVA intracommunautaire doit etre collecte pour les clients B2B EU.
- Le reverse charge doit etre supporte quand applicable.
- Stripe Tax ou mecanisme equivalent doit etre configure avant lancement payant.
- Les factures doivent etre conservees selon les obligations comptables applicables.
