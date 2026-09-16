# Base sécurité et confidentialité — socle V1

## Portée

Cette base s'applique à Identity, Account, Billing, Email, Trust/Risk et Platform
Operations. Elle protège un service B2C tout public et les équipes sans promettre
de conformité Enterprise ou de certification. La
[direction produit](../product/nvbes-product-strategy.md) prévaut.

Les exigences propres au stockage de fichiers, aux liens publics, à Cloud/Drive,
aux organisations Enterprise ou à une API produit sont futures.

## Principes

- privacy et sécurité by design ;
- minimisation des données et des privilèges ;
- frontières de service et sources de vérité distinctes ;
- défense en profondeur proportionnée au risque ;
- audit des actions sensibles ;
- procédures manuelles sûres avant automatisation ;
- coûts de sécurité inclus dans le plafond global de 30 EUR TTC ;
- aucune affirmation de certification sans preuve externe valide.

## Sécurité V1

- TLS sur tous les flux ;
- Cloudflare pour edge, WAF et rate limiting de base ;
- bases et sauvegardes chiffrées ;
- secrets par service, injectés hors code et images ;
- rotation documentée des credentials ;
- MFA obligatoire pour opérateur et accès infrastructure ;
- step-up pour toute commande sensible ;
- comptes nominatifs, aucun compte opérateur partagé ;
- moindre privilège et séparation lecture/action ;
- logs structurés sans secrets ni données personnelles inutiles ;
- audit append-only des décisions et commandes ;
- restauration isolée démontrée avant comptes invités ;
- dépendances, images et artefacts vérifiés par les gates du dépôt.

## Identity et Account

- vérification email avant les usages sensibles ;
- récupération avec jeton court, usage unique et expiration ;
- protection contre brute force et credential stuffing ;
- sessions expirantes, révocables et rotatives ;
- invalidation après compromission ou changement critique ;
- email jamais utilisé comme identifiant d'autorisation codé en dur ;
- équipes B2C avec permissions serveur explicites ;
- SSO/SAML, SCIM et gouvernance Enterprise hors V1.

## Billing

- paiement délégué au PSP ;
- aucune conservation de PAN ou CVV ;
- webhooks signés, idempotents et durablement enregistrés ;
- timeout de paiement traité comme `pending`, jamais comme succès ;
- réconciliation avant activation de paiements réels ;
- opérations ambiguës ou exceptionnelles revues manuellement.

## Email

- acceptation durable avant envoi ;
- retries bornés et dead-letter ;
- webhooks fournisseur authentifiés et idempotents ;
- suppressions et événements de délivrabilité traçables ;
- aucune campagne marketing ou préférence center dans le scope V1.

## Trust/Risk et abus

- signaux minimisés et pseudonymisés lorsque possible ;
- évaluations déterministes et explicables ;
- shadow mode lors de la première production ;
- aucun blocage irréversible automatique ;
- décision finale conservée par le domaine propriétaire ;
- recours et motif enregistrés dans Platform Operations.

## Platform Operations

- `platform_owner` attribué à un opérateur identifié ;
- données personnelles masquées par défaut ;
- aucun accès direct aux bases des services ;
- aucune usurpation complète de session utilisateur ;
- commande sensible motivée et idempotente ;
- suspension temporaire préférée à la suppression ;
- suppression différée, annulable et protégée par un nouveau step-up ;
- résultat incertain conservé comme `pending` ou `unknown`.

Le [runbook opérateur solo](../operations/platform-operations-manual-runbook.md)
définit le traitement des dossiers.

### Incident Response Operationnel

En cas d'incident de sécurité ou d'abus avéré, l'opérateur applique le runbook d'incident, isole les accès concernés, consigne les actions menées et notifie selon les délais légaux applicables.

### Exigences Audit V1

Les événements d'audit enregistrent de manière immuable les créations, mutations de statut, élévations de privilèges et révocations de session sans fuite de secrets ou de données sensibles.

## Privacy V1

- registre des traitements et sous-traitants réels ;
- finalité et durée de conservation explicites ;
- export, rectification et suppression des données applicables ;
- suppression rejouée après restauration lorsque nécessaire ;
- procédure de violation de données personnelles ;
- analytics non essentiels désactivés sans base légale ou consentement requis ;
- séparation entre logs techniques, audit et analytics ;
- aucune donnée sensible dans la télémétrie non prévue pour elle.

## FinOps sécurité

Les contrôles critiques locaux et CI peuvent être automatisés lorsqu'ils ne
créent pas de coût récurrent significatif. Les services externes payants,
collecteurs permanents, bug bounties et audits de certification restent futurs
jusqu'à financement.

L'absence d'un service payant ne permet pas de supprimer un contrôle nécessaire :
utiliser une procédure manuelle ou une solution maison sûre, ou maintenir le
parcours fermé si le risque ne peut pas être maîtrisé.

## Gate de production

- checks sécurité, secrets, dépendances et FinOps verts ;
- aucun défaut critique connu ;
- runbooks et alertes du service présents ;
- restauration réussie ;
- accès opérateur et step-up testés ;
- coût prévisionnel inférieur ou égal à 20 EUR TTC ;
- pire scénario borné inférieur ou égal à 30 EUR TTC ;
- aucune garantie de conformité, disponibilité ou support non démontrée.
