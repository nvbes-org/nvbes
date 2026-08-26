# Politique Commerciale et de Facturation

> **Statut : modèle commercial futur, hors V1 active.** Aucun abonnement,
> renouvellement, plan Cloud/Drive ou paiement réel n'est autorisé par ce
> document. Les règles ci-dessous devront être recalculées et validées pour le
> premier produit effectivement sélectionné, avec distinction B2C/B2B et revue
> juridique/fiscale applicable.

## 1. Abonnements et Renouvellements

### 1.1 Cycle de facturation
Les services nvbes sont fournis sur une base d'abonnement recurrent (mensuel ou annuel). La facturation intervient au début de chaque période de service.

### 1.2 Renouvellement automatique
Sauf résiliation par le Client avant la fin de la période en cours via l'interface de gestion (Billing Portal), l'abonnement est renouvelé automatiquement pour une période identique.

### 1.3 Evolution de plan (Upgrade/Downgrade)
- **Upgrade**: Le passage à un plan supérieur est immédiat. Un prorata est calculé et facturé pour la période restante.
- **Downgrade**: Le passage à un plan inférieur ou gratuit (Trial/Free) prend effet à la fin de la période de facturation en cours. Aucun remboursement n'est effectué pour la période entamée.

## 2. Paiements et Incidents

### 2.1 Moyens de paiement
nvbes accepte les paiements par carte bancaire via son prestataire Stripe. Les informations de paiement sont stockées de manière sécurisée par Stripe.

### 2.2 Défaut de paiement
En cas d'échec de paiement (carte expirée, solde insuffisant, etc.) :
1. **Tentatives de recouvrement**: Stripe effectue plusieurs tentatives de prélèvement sur une période de 14 jours.
2. **Statut 'Past Due'**: Le workspace passe en statut `past_due`. L'accès aux fonctionnalités d'administration et de modification peut être restreint.
3. **Grace Period**: nvbes accorde une période de grâce de 14 jours avant suspension effective du service.
4. **Suspension**: Si le paiement n'est pas régularisé après 14 jours, l'accès au service est suspendu.

## 3. Résiliation et Suppression des données

### 3.1 Modalités de résiliation
Le Client peut résilier son abonnement à tout moment via le Billing Portal. La résiliation prend effet à la date d'échéance de la période en cours.

### 3.2 Conservation après résiliation
À l'issue de la période contractuelle (fin d'abonnement ou résiliation effective) :
1. **Accès en lecture seule**: Le workspace peut rester accessible en lecture seule pendant 30 jours pour permettre l'export des données.
2. **Suppression définitive**: Sauf obligation légale de conservation, les données (fichiers et métadonnées) sont supprimées définitivement 30 jours après la fin du contrat.
3. **Workspaces Inactifs**: Les workspaces gratuits ou en période d'essai expirée n'ayant enregistré aucune activité pendant 90 jours sont supprimés automatiquement.

## 4. Remboursements et Crédits

### 4.1 Politique de non-remboursement
Sauf disposition légale impérative ou erreur manifeste de facturation de la part de nvbes, les sommes versées ne sont pas remboursables.

### 4.2 Crédits commerciaux
nvbes peut, à sa discrétion, octroyer des crédits commerciaux en cas d'incident de service majeur. Ces crédits sont applicables sur les factures futures et ne sont pas convertibles en numéraire.

## 5. Fiscalité

### 5.1 TVA et Autoliquidation
- **Clients FR**: La TVA française (20%) est appliquée.
- **Clients UE (B2B)**: Si un numéro de TVA intracommunautaire valide est fourni, la facturation s'effectue hors taxes (Reverse Charge / Autoliquidation).
- **Clients Hors UE**: La facturation s'effectue hors taxes, sous réserve des règles locales applicables.

## 6. Contact Facturation
Pour toute question relative à la facturation : `billing@nvbes.eu`
