# Runbook — Email Retardé ou Backlog d'Envoi

## 1. Contexte et Détection

- **Sévérité** : SEV2
- **Composant impacté** : `email-worker` / Adaptateur Scaleway Transactional Email (TEM)
- **Signaux d'alerte** :
  - File d'attente `email_messages` en croissance continue (`queued_message_count > 50`).
  - Taux de livraison 24h en baisse ou retards > 5 minutes sur les emails transactionnels de récupération de mot de passe.
  - Événements `email_deferred` ou `email_soft_bounced` reçus en masse depuis Scaleway TEM.

## 2. Comportement Dégradé Automatique

- **Persistance garantie avant expédition** : toute commande d'envoi est écrite en base PostgreSQL de manière transactionnelle avant tentative de délivrance.
- **Retries bornés avec backoff** : le worker réessaie les envois temporairement rejetés avec backoff exponentiel (max 5 tentatives).
- **Dead-letter après épuisement** : les emails définitivement non délivrables basculent en `dead_letter` sans bloquer le reste de la file d'attente.
- **L'échec d'envoi n'annule pas la transaction métier** : la création de compte ou l'action utilisateur reste valide.

## 3. Procédure d'Intervention Opérateur

1. **Vérifier l'état de l'API Scaleway TEM** :
   - Consulter le statut des services Scaleway (https://status.scaleway.com).
   - Vérifier les quotas journaliers d'envoi d'emails (quota FinOps : 100 emails/jour en V1).
2. **Examiner les suppressions actives** :
   - Dans le cockpit Platform Operations, inspecter le panneau `Active Suppressions`.
   - Vérifier si des adresses légitimes ont été bloquées suite à un hard bounce ou une fausse plainte spam.
3. **Rejeu manuel audité** :
   - Si un email critique (ex. code d'invitation ou lien de récupération) a échoué suite à une panne réseau transitoire :
     - Utiliser le bouton `Replay Queued Email` dans le cockpit avec l'identifiant du message et un motif explicite.
4. **Levée de suppression manuelle** :
   - Utiliser `Release Suppression` avec le motif documenté après confirmation de l'utilisateur.

## 4. Vérification et Clôture

- Déclencher un envoi synthétique de test :
  ```bash
  cargo test -p nvbes-email-worker --test synthetic_smoke
  ```
- Vérifier que le temps moyen d'attente dans la file retombe sous 30 secondes.
- Confirmer que le snapshot Email du cockpit affiche zéro événement non traité en souffrance.
