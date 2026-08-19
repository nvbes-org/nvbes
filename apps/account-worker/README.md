# nvbes Account Worker

Worker asynchrone pour le produit nvbes Account.

## Responsabilités

- Exécution des sagas durables d'orchestration (fermeture de compte, exports RGPD multi-produits).
- Consommation des événements `account.closure.requested.v1` et exécution ordonnée des checkpoints (Cloud, Billing, Identity, Account).
- Collecte des fragments d'export de données personnelles et génération du document d'archive sécurisé.
- Traitement de l'outbox / inbox des événements Account.

## Commandes

```bash
# Lancement local
pnpm dev:account-worker

# Vérification Cargo
cargo check -p nvbes-account-worker
```
