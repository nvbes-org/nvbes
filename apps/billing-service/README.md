# nvbes Billing Service

Service central de facturation et de gestion des abonnements pour l'écosystème nvbes.

## Responsabilités

- Gestion du catalogue des plans, abonnements et factures.
- Intégration et réconciliation des webhooks PSP (Stripe, etc.).
- Calcul des estimations et exposition des soldes / quotas via gRPC et API REST.
- Ledger financier append-only pour les écritures comptables et les avoirs.

## Commandes

```bash
# Lancement local
pnpm dev:billing-service

# Vérification Cargo
cargo check -p nvbes-billing-service
```
