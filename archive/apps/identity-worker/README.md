# nvbes Identity Worker

Worker asynchrone pour la maintenance et les opérations d'arrière-plan d'Identity Service.

## Responsabilités

- Nettoyage des sessions expirées, jetons de réinitialisation et tokens de vérification non consommés.
- Traitement de l'outbox des événements d'authentification et d'audit.
- Maintenance périodique des clés de signature et des défis WebAuthn expirés.

## Commandes

```bash
# Lancement local
pnpm dev:identity-worker

# Vérification Cargo
cargo check -p nvbes-identity-worker
```
