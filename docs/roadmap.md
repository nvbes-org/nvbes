# Roadmap nvbes

## Autorité

Cette roadmap applique la [direction produit V1](product/nvbes-product-strategy.md).
Les anciennes roadmaps Cloud, Drive, Developer ou Enterprise sont des
hypothèses futures et ne constituent plus des lots actifs.

## V1 finale — socle réutilisable

Objectif : livrer un socle B2C tout public, compatible avec les équipes, sans
produit final ni offre B2B, exploitable progressivement par un opérateur solo
pour un coût récurrent total inférieur ou égal à 30 EUR TTC par mois.

### Gate transversal permanent

- aucune dette technique ou structurelle connue dans le périmètre livré ;
- cible de coût à 20 EUR TTC et limite dure à 30 EUR TTC ;
- administration, support et modération manuels par défaut ;
- automatisation seulement après besoin mesuré et preuve coût/sécurité ;
- solutions maison possibles si elles sont sûres, maintenables et remplaçables ;
- déploiements progressifs, observables et réversibles ;
- aucune dépendance du socle envers Cloud, Drive ou un autre produit final.

### Lot 1 — FinOps et primitives communes

- contrat budgétaire exécutable et gate CI ;
- plafonds serverless et scale-to-zero ;
- contrats d'audit, idempotence, outbox, retries et dead-letter ;
- corrélation, métriques, erreurs et modes dégradés ;
- procédure manuelle de revue des coûts TTC.

Sortie : le coût prévisionnel reste sous 20 EUR et le pire scénario autorisé
reste sous 30 EUR.

### Lot 2 — Email

- acceptation durable des commandes ;
- rendu, livraison, retries, suppressions et événements fournisseur ;
- observabilité, runbook et restauration ;
- ouverture progressive sans dépendance à un produit final.

Sortie : parcours transactionnel synthétique livré et traçable, sans envoyer de
message réel lorsque le smoke est désactivé.

### Lot 3 — Identity

- authentification, credentials, sessions, récupération, MFA et step-up ;
- autorisation interservices et contrats réutilisables ;
- accès interne avant comptes invités.

Sortie : compte synthétique authentifié, récupérable et audité.

### Lot 4 — Account et équipes

- profil, préférences et cycle de vie du compte ;
- équipes et memberships comme primitives B2C collaboratives ;
- export et suppression des données applicables ;
- aucune console ou gouvernance Enterprise.

Sortie : un compte peut créer et rejoindre une équipe sans dépendre d'un
produit final.

### Lot 5 — Billing

- catalogue, prix, abonnements, paiements et entitlements minimaux ;
- fournisseur de paiement derrière un contrat remplaçable ;
- webhooks, idempotence et réconciliation en mode test ;
- opérations financières manuelles lorsque l'automatisation n'est pas justifiée.

Sortie : parcours fournisseur de test réconcilié ; aucun paiement réel avant
validation distincte.

### Lot 6 — Trust/Risk

- signaux transversaux et recommandations explicables ;
- jeux de référence déterministes ;
- déploiement en shadow mode ;
- décision d'enforcement conservée par le domaine propriétaire.

Sortie : recommandation shadow traçable, sans blocage automatique prématuré.

### Lot 7 — Platform Operations

Implémentation et preuve locale : [API et parcours opérateur](operations/platform-operations-api.md).
Les commandes de dossier et de registre sont persistées et auditées ; les
interventions métier restent manuelles dans le domaine propriétaire, avec les
gates de sécurité décrits dans le runbook.

- dossiers support, sécurité, abus, facturation et recours ;
- contexte consolidé sans accès direct aux bases des services ;
- commandes opérateur idempotentes, motivées et auditées ;
- registre de coûts et vue des audits ;
- rôle initial `platform_owner`, préparé pour une séparation future des rôles.

Sortie : l'opérateur solo peut traiter manuellement un dossier de bout en bout
sans usurpation de session ni action irréversible non protégée.

### Lot 8 — validation intégrée et ouverture progressive

1. exécuter le parcours Identity, Account, Billing, Email, Trust/Risk et
   Platform Operations ;
2. démontrer sauvegarde et restauration ;
3. vérifier coûts, quotas, cold starts et modes dégradés ;
4. ouvrir à un petit groupe de comptes invités ;
5. mesurer demandes, abus, incidents et coût humain ;
6. autoriser les inscriptions publiques uniquement après un GO explicite.

## Après V1 — sélection du premier produit

Le premier produit est choisi par une décision distincte selon la valeur
utilisateur, le coût total, la charge opérateur et la réutilisation du socle.
Cloud/Drive reste un candidat, pas une priorité implicite.

Le produit sélectionné doit réutiliser les services du socle sans déplacer leur
source de vérité. Son marketing, pricing, beta, stockage et modération propre ne
deviennent actifs qu'après cette décision.

## Futures versions

### Évolution produit et équipes

- capacités propres au premier produit validé ;
- automatisations déclenchées par des volumes mesurés ;
- ajout d'opérateurs avec séparation réelle des permissions ;
- optimisation des coûts financée par les revenus ou une économie démontrée.

### B2B et Enterprise

- contrats entreprise et SLA ;
- SSO/SAML, SCIM et gouvernance avancée ;
- conformité, certifications et audits externes ;
- support structuré et rôles opérateur spécialisés.

Ces travaux restent hors périmètre tant que le budget, les revenus et la demande
ne permettent pas de les opérer correctement.

### Échelle globale

Multi-région, multi-cloud, Kubernetes, data platforms et hyperscale sont des
explorations long terme. Ils ne sont activés que par des seuils de charge,
risque ou revenus mesurés.

## NO-GO

- lancer Cloud/Drive parce que le code ou un ancien PRD existe ;
- traiter la V1 comme une offre B2B ou promettre une conformité Enterprise ;
- dépasser 30 EUR TTC de coût récurrent total ;
- automatiser une demande encore rare sans justification mesurée ;
- ajouter une solution payante alors qu'une procédure manuelle sûre suffit ;
- accepter une dette provisoire dans le périmètre livré ;
- ouvrir publiquement avant les preuves de restauration, coût et exploitation.
