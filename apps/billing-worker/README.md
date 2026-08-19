# nvbes Billing Worker

Worker asynchrone pour le service Billing.

## Responsabilités

- Traitement asynchrone des événements de facturation et réconciliation des paiements.
- Exécution des tâches périodiques de relance, calcul d'usage et clôture de cycles.
- Notification des renouvellements et alertes de paiement échoué.

## Commandes

```bash
# Lancement local
pnpm dev:billing-worker

# Vérification Cargo
cargo check -p nvbes-billing-worker
```
