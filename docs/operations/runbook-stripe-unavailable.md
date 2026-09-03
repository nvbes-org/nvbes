# Runbook — Stripe API / Webhooks Indisponible

## 1. Contexte et Détection

- **Sévérité** : SEV2
- **Composant impacté** : `billing-service` / Intégration Stripe test
- **Signaux d'alerte** :
  - Échecs de validation des signatures de webhooks (`whsec_test_*`).
  - Timeout ou 5xx lors des appels de création de Stripe Checkout Session test.
  - Alerte `billing-reconciliation-mismatch` dans le cockpit Platform Operations.

## 2. Règles Impératives et Comportement Dégradé

- **Zéro paiement réel en V1** : toute clé `sk_live_` ou `whsec_live_` est rejetée par assertion stricte.
- **Aucune rupture de service automatique** : un échec d'ingestion webhook ne révoque JAMAIS automatiquement l'accès d'un utilisateur ou d'une équipe.
- **État `pending` conservé** : tout paiement dont le résultat est incertain est conservé en état `pending` dans la table locale d'abonnements jusqu'à réconciliation manuelle par l'opérateur.
- **Tolérance temporelle des webhooks** : Stripe applique des retries exponentiels pendant 72 heures ; les événements reçus avec retard sont dédupliqués par `event.id`.

## 3. Procédure d'Intervention Opérateur

1. **Vérifier l'état de l'API Stripe** :
   - Consulter la page de statut officiel de Stripe (https://status.stripe.com).
2. **Vérifier les secrets de webhook de test** :
   - S'assurer que `NVBES_STRIPE_WEBHOOK_SECRET` correspond au secret configuré dans le dashboard Stripe test.
   - S'assurer que l'horodatage système est synchronisé (tolérance de signature de 300 secondes).
3. **Inspecter la dead-letter queue Billing** :
   - Depuis le cockpit Platform Operations, examiner les événements webhook échoués.
   - Analyser le motif de rejet (`signature_mismatch`, `payload_parse_error`, `desync`).
4. **Réconciliation manuelle auditée** :
   - Si un abonnement de test nécessite une régularisation, l'opérateur utilise l'action `reconcile_billing_event` avec justification obligatoire (min 3 caractères).
   - Aucun accès direct en base de données n'est toléré.

## 4. Vérification et Clôture

- Envoyer un événement de test signé via les outils de test Stripe CLI.
- Vérifier que `billing_webhook_events` enregistre l'événement avec l'état `processed`.
- Confirmer que le compteur d'écarts de réconciliation dans le cockpit Platform Operations revient à zéro.
