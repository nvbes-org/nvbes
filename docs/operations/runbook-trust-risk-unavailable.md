# Runbook — Trust/Risk Service Indisponible

## 1. Contexte et Détection

- **Sévérité** : SEV3
- **Composant impacté** : `trust-risk-service`
- **Signaux d'alerte** :
  - Échec de connexion gRPC vers le port interne de `trust-risk-service`.
  - Sonde `/health/live` en échec sur le conteneur Scaleway Trust/Risk.
  - Les producteurs d'événements (`identity-service`, `account-service`) enregistrent des timeouts gRPC lors des évaluations de signaux.

## 2. Comportement Dégradé Automatique

- **Shadow mode par défaut** : en V1, Trust/Risk n'applique AUCUN blocage automatique. Ses évaluations sont purement consultatives pour l'opérateur.
- **Fail-open explicite (`allow`)** : les services appelants basculent immédiatement en mode dégradé `allow` pour les parcours utilisateur légitimes.
- **Persistance des signaux bruts** : les signaux de contexte (IP, User-Agent, horodatage) sont persistés dans les logs d'audit append-only des services producteurs afin de permettre une évaluation a posteriori une fois Trust/Risk restauré.
- **Aucune fausse alerte utilisateur** : aucun utilisateur n'est bloqué ou banni par défaut en cas d'indisponibilité de Trust/Risk.

## 3. Procédure d'Intervention Opérateur

1. **Vérifier l'instance Scaleway Trust/Risk** :
   ```bash
   scw container container get <TRUST_RISK_CONTAINER_ID>
   ```
2. **Vérifier la base PostgreSQL Trust/Risk** :
   - Tester la connectivité à `trust_risk_production`.
   - Vérifier l'espace disque et la table `trust_risk_evaluations`.
3. **Consulter les métriques de latence gRPC** :
   - Vérifier si la panne est causée par un cold start ou une saturation de requêtes.
4. **Relance du service** :
   - Redémarrer le conteneur avec `min_scale=0` / `max_scale=1`.

## 4. Vérification et Clôture

- Vérifier que la sonde liveness HTTP répond 200 sur le conteneur.
- Exécuter la validation des règles Trust/Risk :
  ```bash
  cargo test -p nvbes-trust-risk-service
  ```
- Confirmer sur le cockpit Platform Operations que le compteur des évaluations shadow reprend son incrémentation normale.
