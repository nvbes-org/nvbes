# API Publique V1 - Upload et Download

> **Statut : capacité Cloud/Drive future, hors socle V1 actif.**

## Upload

Le flux V1 cree une session d'upload cote Drive, retourne une URL courte, puis active l'objet apres completion.

Sequence:

1. `POST /v1/workspaces/:workspaceId/uploads`
2. upload direct vers l'URL retournee;
3. `POST /v1/workspaces/:workspaceId/uploads/:uploadId/complete`

## Creation d'Upload

Requete:

```http
POST /v1/workspaces/:workspaceId/uploads
Authorization: Bearer <token>
Idempotency-Key: <uuid>
Content-Type: application/json
```

Payload:

```json
{
  "parent_id": null,
  "name": "contrat.pdf",
  "mime_type": "application/pdf",
  "expected_size_bytes": 1048576,
  "expected_checksum": "hex-encoded-sha256"
}
```

Reponse:

```json
{
  "upload_id": "uuid",
  "storage_object": {
    "id": "uuid",
    "status": "pending"
  },
  "upload_url": {
    "url": "https://...",
    "method": "PUT",
    "expires_at": "2026-06-05T12:30:00Z",
    "required_headers": []
  },
  "tus_url": "/workspaces/{workspaceId}/uploads/{uploadId}",
  "expires_at": "2026-06-05T12:30:00Z",
  "required_headers": []
}
```

Regles:

- `expected_size_bytes` est obligatoire et compte dans la reservation quota.
- `upload_url` expire rapidement.
- l'objet reste `pending` tant que la completion n'a pas valide taille et checksum.
- une completion reussie est idempotente avec la meme `Idempotency-Key`.

## Completion

Requete:

```http
POST /v1/workspaces/:workspaceId/uploads/:uploadId/complete
Authorization: Bearer <token>
Idempotency-Key: <uuid>
Content-Type: application/json
```

Payload:

```json
{
  "size_bytes": 1048576,
  "checksum": "hex-encoded-sha256"
}
```

Reponse:

```json
{
  "upload_id": "uuid",
  "status": "active",
  "storage_object": {
    "id": "uuid",
    "name": "contrat.pdf",
    "size_bytes": 1048576,
    "status": "active"
  },
  "activated_at": "2026-06-05T12:05:00Z"
}
```

## Annulation

`POST /v1/workspaces/:workspaceId/uploads/:uploadId/cancel` libere la reservation quota si l'upload n'est pas encore actif.

## Download

Le download public V1 ne transmet jamais d'object key. Le client demande une URL courte apres verification des permissions.

Requete:

```http
POST /v1/workspaces/:workspaceId/objects/:objectId/download-url
Authorization: Bearer <token>
Content-Type: application/json
```

Payload:

```json
{}
```

Reponse:

```json
{
  "object_id": "uuid",
  "download_url": {
    "url": "https://...",
    "method": "GET",
    "expires_at": "2026-06-05T12:10:00Z"
  }
}
```

Regles:

- URL courte duree;
- audit event obligatoire;
- egress comptabilise;
- refus si objet pending, trashed, deleted ou quarantine.
