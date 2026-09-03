# Runbook — Account Service Indisponible

## 1. Contexte et Détection

- **Sévérité** : SEV2
- **Composant impacté** : `account-service`
- **Signaux d'alerte** :
  - Sondes `/health/live` ou `/health/ready` en échec sur `account-service`.
  - Échecs des mutations de profil, préférences ou appartenances aux équipes.
  - Retards ou blocages sur les jobs de privacy / export RGPD.

## 2. Comportement Dégradé Automatique

- **Isolation de domaine** : l'authentification Identity reste fonctionnelle ; les utilisateurs peuvent se connecter même si leur profil Account est temporairement indisponible.
- **Mutations suspendues** : toute création ou modification d'équipe renvoie HTTP 503 explicite.
- **Exports RGPD** : les demandes d'export sont persistées dans l'outbox pour traitement ultérieur dès rétablissement, sans perte de données.
- **Aucune décision automatique** : aucun compte n'est suspendu ou clôturé automatiquement pendant la panne.

## 3. Procédure d'Intervention Opérateur

1. **Vérifier l'état de l'instance serverless Account** :
   ```bash
   scw container container get <ACCOUNT_CONTAINER_ID>
   ```
2. **Vérifier les migrations et le verrouillage de base** :
   - Vérifier si une migration SQLx est bloquée sur `account_service`.
   - Inspecter les métriques de la base PostgreSQL dédiée Account (`nvbes-account-production`).
3. **Consulter l'outbox Account** :
   - Identifier si des événements asynchrones sont accumulés en échec (`account.user.created`, `account.billing.facade.requested`).
4. **Action de remédiation** :
   - Si saturation mémoire : redémarrage contrôlé de l'instance container.
   - Si régression logicielle : rollback vers le digest conteneur précédent validé.

## 4. Vérification et Clôture

- Vérifier que `/health/ready` répond HTTP 200.
- Exécuter la suite de tests de contrat de base de données :
  ```bash
  bash scripts/test-account-service-database.sh
  ```
- Confirmer sur le cockpit Platform Operations que le compteur des jobs outbox en attente diminue.
- Clôturer le dossier d'incident dans le registre des opérations.
