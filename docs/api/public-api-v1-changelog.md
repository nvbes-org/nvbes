# API Publique V1 - Changelog

> **Statut : changelog historique d'une API produit inactive.**

## Regles

- Toute release publique modifiant `/v1` ajoute une entree ici.
- Les changements sont classes en `Added`, `Changed`, `Deprecated`, `Removed`, `Fixed`, `Security`.
- `Removed` est interdit dans `/v1` sauf retrait d'un comportement deja sunset et documente.
- Les changements incompatibles exigent une nouvelle version majeure d'API.

## 2026-06-05 - V1 Contract Freeze

Added:

- contrat initial des endpoints `/v1`;
- guide authentication et scopes;
- guide upload et download;
- guide erreurs, rate limits et idempotence;
- exemples curl;
- politique de compatibilite.

Known release blockers:

- publier une OpenAPI versionnee contenant les routes `/v1`;
- brancher les smoke tests API publique sur auth, scopes, upload, download, erreurs et rate limits;
- confirmer les limites finales par plan avec FinOps.
