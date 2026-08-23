# nvbes Enterprise Service

Service backend dédié aux fonctionnalités d'administration d'entreprise et de gouvernance multi-tenants.

## Responsabilités

- Gestion des politiques de sécurité organisationnelles (MFA obligatoire, SSO / SAML, SCIM).
- Gestion des domaines vérifiés et des fédérations d'identités d'entreprise.
- Journaux d'audit avancés et rapports de conformité.

## Commandes

```bash
# Lancement local
pnpm dev:enterprise-service

# Vérification Cargo
cargo check -p nvbes-enterprise-service
```
