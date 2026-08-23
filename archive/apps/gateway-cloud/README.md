# nvbes Gateway Cloud

Passerelle API et couche BFF (Backend-For-Frontend) pour les services Cloud nvbes.

## Responsabilités

- Point d'entrée unifié pour le frontend Cloud (`cloud-web`).
- Vérification et introspection des tokens d'accès OAuth.
- Routage et composition des requêtes vers `cloud-service`, `account-service` et `billing-service`.
- Application des politiques de rate limiting et de sécurité HTTP.

## Commandes

```bash
# Lancement local
pnpm dev:gateway-cloud

# Vérification Cargo
cargo check -p nvbes-gateway-cloud
```
