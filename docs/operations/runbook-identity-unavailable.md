# Runbook — Identity Service Indisponible

## 1. Contexte et Détection

- **Sévérité** : SEV1
- **Composant impacté** : `identity-service`
- **Signaux d'alerte** :
  - Échecs des sondes `/health/live` et `/health/ready` sur `identity-service`.
  - Augmentation des erreurs 502/503/504 en bordure (Cloudflare) sur les routes `/auth/*`.
  - Downstream services (`account-service`, `billing-service`) signalent des échecs de validation des clés publiques JWT ou de dérivation des tokens.

## 2. Comportement Dégradé Automatique

- **Pas d'enforcement automatique** : les downstream services ne doivent jamais contourner la validation des JWT.
- **Sessions existantes** : les tokens JWT RS256 valides non expirés continuent d'être acceptés par les runtimes sans introspection synchrone (fenêtre de 15 minutes max pour la révocation).
- **Nouvelles authentifications** : passage en lecture seule strict pour les utilisateurs non authentifiés ; refus explicite des demandes d'authentification et de réinitialisation de mot de passe avec HTTP 503.

## 3. Procédure d'Intervention Opérateur

1. **Diagnostic du conteneur Scaleway** :
   ```bash
   scw container container get <CONTAINER_ID>
   ```
   Vérifier si le conteneur subit un crashloop ou une erreur de mémoire/timeout.
2. **Vérification de la base PostgreSQL Identity** :
   - Tester la connectivité de la base serverless via son endpoint privé.
   - S'assurer que le pool de connexions (`NVBES_IDENTITY_DATABASE_MAX_CONNECTIONS=5`) n'est pas saturé.
3. **Vérification des variables et secrets** :
   - Confirmer la présence de `NVBES_IDENTITY_MFA_ENCRYPTION_KEY`.
   - Confirmer la validité de la paire de clés RS256 d'émission JWT.
4. **Redémarrage ou Rollback** :
   - Redémarrer l'instance serverless si le crashloop est passager.
   - Effectuer un rollback immédiat vers le dernier digest d'image immuable vérifié en cas d'incident post-déploiement.

## 4. Vérification et Clôture

- Vérifier que `/health/live` et `/health/ready` répondent HTTP 200.
- Exécuter le test de validation synthétique :
  ```bash
  bash tools/deployment/prove-identity-synthetic-auth.sh
  ```
- Confirmer la baisse du taux d'erreurs 5xx sur le cockpit Platform Operations.
- Enregistrer le dossier d'incident dans le registre d'audit avec motif et heure de résolution.
