# API Publique V1

> **Statut : contrat produit Cloud/Drive futur, hors V1 active.** Aucune API
> publique de fichiers n'est lancée avec le socle. Voir la
> [direction produit](../product/nvbes-product-strategy.md).

## Statut de Verrouillage

Statut: contract freeze documentaire, non publie public.

La publication publique est bloquee tant que:

- `apps/cloud-service/openapi.json` ne contient pas les routes `/v1` listees ci-dessous;
- la spec OpenAPI publique n'est pas archivee comme artefact versionne de release;
- les guides operateurs et developpeurs ci-dessous ne sont pas relus avec les contrats backend reels;
- les smoke tests API publique couvrent auth, scopes, upload, download, erreurs, rate limits et idempotence.

Guides de reference:

- [Authentification et scopes](public-api-v1-auth-scopes.md)
- [Upload et download](public-api-v1-upload-download.md)
- [Erreurs et rate limits](public-api-v1-errors-rate-limits.md)
- [Exemples curl](public-api-v1-curl-examples.md)
- [Changelog](public-api-v1-changelog.md)
- [Compatibilite et non-breaking policy](public-api-v1-compatibility.md)

Artefacts attendus avant lancement public:

- OpenAPI canonique generee: `apps/cloud-service/openapi.json`
- OpenAPI publique versionnee: `docs/api/openapi/drive-public-v1.openapi.json`
- Changelog public: [public-api-v1-changelog.md](public-api-v1-changelog.md)

## Decision

nvbes Drive expose une API publique des la V1.

La V1 est volontairement limitee aux integrations fichiers:

- Lecture fichiers/dossiers.
- Creation de dossiers.
- Upload via signed URL.
- Download via signed URL.
- Creation et revocation de liens partages.
- Lecture quota.
- Lecture audit basique.

## Principes

- Versioning obligatoire via `/v1`.
- API REST.
- API keys legacy scopees par workspace, en lecture/revocation uniquement.
- Nouvelle creation d'API key desactivee: les integrations machine doivent passer par Identity `service accounts` + OAuth clients.
- API keys hashees en base.
- API keys affichees une seule fois lors de la migration ou de la creation historique.
- Scopes obligatoires.
- Rate limits par plan.
- Audit events sur actions sensibles.
- Documentation OpenAPI obligatoire avant lancement public.

## Authentification

Header:

```http
Authorization: Bearer gx_live_xxxxxxxxx
```

Formats:

- `gx_live_...` pour production.
- `gx_test_...` pour environnements non-production.

Regles:

- La cle complete n'est jamais stockee en clair.
- Un prefixe public permet l'identification de la cle.
- La cle peut etre revoquee immediatement.
- Rotation possible depuis l'interface.
- Expiration optionnelle selon plan.
- Derniere utilisation visible dans l'interface.

## Scopes V1

| Scope               | Description                                     |
| ------------------- | ----------------------------------------------- |
| `files:read`        | Lire dossiers, fichiers et metadata.            |
| `files:write`       | Creer dossiers et uploader fichiers.            |
| `files:delete`      | Mettre des fichiers en corbeille uniquement.    |
| `share_links:read`  | Lire les liens partages du workspace.           |
| `share_links:write` | Creer, modifier et revoquer des liens partages. |
| `quota:read`        | Lire les quotas et usages.                      |
| `audit:read`        | Lire les audit events accessibles.              |

Hors V1:

- Suppression definitive.
- Billing mutations.
- Gestion complete des membres.
- Changement des permissions.
- Webhooks publics.
- SDK officiel.
- Admin/security settings.

## Endpoints Publics V1

```http
GET    /v1/me
GET    /v1/workspaces
GET    /v1/workspaces/:workspaceId/objects?parentId=
POST   /v1/workspaces/:workspaceId/folders
PATCH  /v1/workspaces/:workspaceId/objects/:objectId
POST   /v1/workspaces/:workspaceId/objects/:objectId/move
POST   /v1/workspaces/:workspaceId/objects/:objectId/trash
POST   /v1/workspaces/:workspaceId/uploads
POST   /v1/workspaces/:workspaceId/uploads/:uploadId/complete
POST   /v1/workspaces/:workspaceId/uploads/:uploadId/cancel
POST   /v1/workspaces/:workspaceId/objects/:objectId/download-url
GET    /v1/workspaces/:workspaceId/share-links
POST   /v1/workspaces/:workspaceId/objects/:objectId/share-links
PATCH  /v1/workspaces/:workspaceId/share-links/:shareLinkId
DELETE /v1/workspaces/:workspaceId/share-links/:shareLinkId
GET    /v1/workspaces/:workspaceId/quota
GET    /v1/workspaces/:workspaceId/audit-events
```

## Rate Limits V1

| Plan      | Requetes/min | Requetes/jour | Cles API legacy |
| --------- | ------------ | ------------- | -------- |
| Solo Pro  | 60           | 1 000         | 1        |
| Team      | 300          | 20 000        | 5        |
| Team Plus | 600          | 100 000       | 20       |

Limites specifiques:

- Upload sessions par heure.
- Download URLs par minute.
- Share links crees par jour.
- Egress mensuel.

Les limites exactes peuvent etre ajustees apres mesure FinOps.

## Erreurs

Format standard:

```json
{
  "error": {
    "code": "quota_exceeded",
    "message": "Le quota de stockage de cet espace est atteint.",
    "requestId": "req_..."
  }
}
```

Codes minimum:

- `invalid_api_key`
- `revoked_api_key`
- `expired_api_key`
- `insufficient_scope`
- `rate_limited`
- `quota_exceeded`
- `object_not_found`
- `upload_expired`
- `workspace_suspended`
- `validation_failed`
- `internal_error`

## Idempotence

Recommandee pour:

- Creation de dossier.
- Creation d'upload session.
- Completion upload.
- Creation lien partage.

Header:

```http
Idempotency-Key: <client-generated-key>
```

## Pagination

Les endpoints de liste doivent supporter:

- `limit`
- `cursor`

Les reponses doivent retourner:

- `items`
- `nextCursor`

## Audit Events

Evenements minimum:

- `api_key.created`
- `api_key.revoked`
- `api_key.rotated`
- `api.request.denied`
- `api.file.uploaded`
- `api.file.download_url_created`
- `api.file.trashed`
- `api.share_link.created`
- `api.share_link.revoked`

## Documentation

Avant lancement public:

- OpenAPI spec versionnee dans `docs/api/openapi/drive-public-v1.openapi.json`.
- Guide authentication et scopes: [public-api-v1-auth-scopes.md](public-api-v1-auth-scopes.md).
- Guide upload et download: [public-api-v1-upload-download.md](public-api-v1-upload-download.md).
- Guide rate limits et erreurs: [public-api-v1-errors-rate-limits.md](public-api-v1-errors-rate-limits.md).
- Exemples curl: [public-api-v1-curl-examples.md](public-api-v1-curl-examples.md).
- Changelog API: [public-api-v1-changelog.md](public-api-v1-changelog.md).
- Politique de compatibilite: [public-api-v1-compatibility.md](public-api-v1-compatibility.md).

## Compatibilite

La politique de reference est [Compatibilite et non-breaking policy](public-api-v1-compatibility.md).
