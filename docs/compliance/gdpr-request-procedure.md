# Procédure de Gestion des Demandes d'Exercice de Droits (RGPD)

## 1. Objectif

Définir les étapes de traitement des demandes formulées par les personnes concernées en vertu des articles 15 à 22 du RGPD (Accès, Rectification, Effacement, Opposition, Limitation, Portabilité).

## 2. Canaux de Réception

Les demandes sont principalement reçues via :
- Email : `privacy@nvbes.eu`
- Formulaire de support dans l'application nvbes.

## 3. Workflow de Traitement

### Étape 1 : Réception et Enregistrement
- Accuser réception de la demande (immédiat).
- Enregistrer la demande dans le registre interne des demandes RGPD.

### Étape 2 : Vérification de l'Identité
- Si le demandeur est authentifié dans l'application, l'identité est présumée valide pour les données liées au compte.
- En cas de doute raisonnable (demande par email externe), demander une pièce d'identité ou une vérification par email lié au compte. *Note : Ne pas conserver la pièce d'identité au-delà de la vérification.*

### Étape 3 : Qualification
- Identifier le périmètre des données concernées.
- Vérifier si nvbes agit en tant que **Responsable de Traitement** (données de compte utilisateur, facturation) ou **Sous-traitant** (données hébergées dans les fichiers du workspace).
- **Si Sous-traitant** : Informer le demandeur que sa demande doit être adressée à l'administrateur du workspace (le Responsable de Traitement) et notifier l'administrateur concerné si possible.

### Étape 4 : Exécution
- **Accès** : Fournir une copie des données personnelles sous un format structuré.
- **Portabilité** : Fournir les données dans un format lisible par machine (JSON/CSV).
- **Effacement** : Procéder à la suppression ou anonymisation des données, sous réserve des exceptions légales (ex: facturation, preuve fiscale).
- **Rectification** : Mettre à jour les données erronées.

### Étape 5 : Réponse et Clôture
- Répondre au demandeur dans un délai maximum de **30 jours**.
- Ce délai peut être prolongé de 60 jours en cas de demande complexe, en informant le demandeur dans le premier mois.

## 4. Registre des Demandes

Le registre doit contenir :
- Date de réception.
- Type de droit exercé.
- Statut (En cours, Traité, Rejeté).
- Date de réponse.
- Justification en cas de rejet (ex: demande manifestement infondée ou excessive).

## 5. Points de Vigilance

- **Données de tiers** : Veiller à ne pas divulguer de données personnelles de tiers lors de l'exercice d'un droit d'accès.
- **Sauvegardes** : Si une suppression est effectuée, s'assurer que la donnée ne sera pas réintroduite lors d'une restauration de backup (voir Politique de Rétention).
- **Validation staging** : Voir le runbook de smoke RGPD sur `staging` dans [docs/operations/gdpr-staging-smoke-runbook.md](/Users/shayn/Development/nvbes/docs/operations/gdpr-staging-smoke-runbook.md).
