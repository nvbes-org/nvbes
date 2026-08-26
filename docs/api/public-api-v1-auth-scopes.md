# API Publique V1 - Authentification et Scopes

> **Statut : annexe d'une future API produit.** Les primitives Identity restent
> actives, mais ces scopes Cloud/Drive ne sont pas une surface V1 ouverte.

## Objectif

Ce guide fige le contrat d'acces public de nvbes Drive V1 pour les integrations fichiers.

## Modes d'Authentification

### Canonique

Les nouvelles integrations machine utilisent Identity:

1. creer un service account dans le workspace;
2. rattacher un client OAuth au service account;
3. obtenir un token via `client_credentials`;
4. appeler Drive avec `Authorization: Bearer <access_token>`.

### Legacy

Les API keys legacy restent supportees pour lecture, rotation et revocation.

La creation de nouvelles API keys legacy est desactivee pour les clients publics V1. Toute exception doit etre documentee dans le changelog et limitee a une migration.

Format:

```http
Authorization: Bearer gx_live_xxxxxxxxx
```

Environnements:

- `gx_live_` pour production;
- `gx_test_` pour development et staging.

## Regles de Securite

- La cle complete n'est jamais stockee en clair.
- La cle complete n'est affichee qu'une seule fois.
- Le prefixe public sert a identifier la cle sans l'exposer.
- Toute cle revoquee doit etre refusee immediatement.
- Les actions sensibles ecrivent un audit event.
- Les erreurs d'authentification ne doivent pas confirmer l'existence d'une cle.

## Scopes

| Scope | Autorise |
| --- | --- |
| `files:read` | Lister objets, lire metadata, generer download URL. |
| `files:write` | Creer dossiers, creer uploads, completer uploads, renommer, deplacer. |
| `files:delete` | Mettre en corbeille uniquement. |
| `share_links:read` | Lister les liens partages du workspace. |
| `share_links:write` | Creer, modifier et revoquer les liens partages. |
| `quota:read` | Lire quotas et usage. |
| `audit:read` | Lire les audit events accessibles a l'integration. |

## Mapping Minimal par Endpoint

| Endpoint | Scope requis |
| --- | --- |
| `GET /v1/me` | `files:read` |
| `GET /v1/workspaces` | `files:read` |
| `GET /v1/workspaces/:workspaceId/objects` | `files:read` |
| `POST /v1/workspaces/:workspaceId/folders` | `files:write` |
| `PATCH /v1/workspaces/:workspaceId/objects/:objectId` | `files:write` |
| `POST /v1/workspaces/:workspaceId/objects/:objectId/move` | `files:write` |
| `POST /v1/workspaces/:workspaceId/objects/:objectId/trash` | `files:delete` |
| `POST /v1/workspaces/:workspaceId/uploads` | `files:write` |
| `POST /v1/workspaces/:workspaceId/uploads/:uploadId/complete` | `files:write` |
| `POST /v1/workspaces/:workspaceId/uploads/:uploadId/cancel` | `files:write` |
| `POST /v1/workspaces/:workspaceId/objects/:objectId/download-url` | `files:read` |
| `GET /v1/workspaces/:workspaceId/share-links` | `share_links:read` |
| `POST /v1/workspaces/:workspaceId/objects/:objectId/share-links` | `share_links:write` |
| `PATCH /v1/workspaces/:workspaceId/share-links/:shareLinkId` | `share_links:write` |
| `DELETE /v1/workspaces/:workspaceId/share-links/:shareLinkId` | `share_links:write` |
| `GET /v1/workspaces/:workspaceId/quota` | `quota:read` |
| `GET /v1/workspaces/:workspaceId/audit-events` | `audit:read` |

## Refus Obligatoires

- cle absente, invalide, expiree ou revoquee;
- token OAuth dont l'audience ne couvre pas Drive;
- workspace absent du contexte autorise;
- scope manquant;
- workspace suspendu;
- plan ou policy qui bloque l'action.
