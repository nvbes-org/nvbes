# nvbes-billing-worker

Worker d'arrière-plan autonome et scale-to-zero pour le traitement asynchrone des événements de facturation (nvbes Billing v1).

## Rôle & Architecture

`nvbes-billing-worker` sépare l'exécution asynchrone lourde du point d'entrée HTTP public `nvbes-billing-service` :

- **Consommation d'événements de file / SQS** : réception des déclenchements Scaleway Container Trigger sur `/internal/queue/billing-dispatch` (ou boucle mpsc en local/tests).
- **Envoi d'e-mails transactionnels** : intégration avec le service gRPC `nvbes-email` pour les reçus de paiement (`billing.invoice.paid.v1`) et alertes d'échec / relance dunning (`billing.invoice.payment_failed.v1`).
- **Synchronisation d'abonnements & Outbox** : traitement idempotent et mise à jour de la table `billing_outbox`.
- **Réconciliation périodique** : déclenchement sur `/internal/reconciliation` par trigger cron Scaleway.
- **FinOps** : conteneur sans état démarrant en scale-to-zero (`min_scale = 0`, `max_scale = 1`), limitant la consommation sous le plafond strict de 30 EUR TTC/mois.

## Endpoints HTTP

- `GET /health/live` : Vérification de vivacité (liveness probe).
- `GET /health/ready` : Vérification de disponibilité (readiness probe).
- `GET /metrics` : Métriques Prometheus protégées par token optionnel.
- `POST /internal/queue/billing-dispatch` : Intake sécurisé déclenché par Scaleway SQS trigger.
- `POST /internal/reconciliation` : Déclenchement de réconciliation cron.

## Commandes CLI

```bash
nvbes-billing-worker serve
nvbes-billing-worker migrate
nvbes-billing-worker validate-runtime
nvbes-billing-worker synthetic-smoke
nvbes-billing-worker deployment-bootstrap
```
