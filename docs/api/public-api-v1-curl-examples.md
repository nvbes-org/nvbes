# API Publique V1 - Exemples curl

## Variables

```bash
export DRIVE_API_BASE_URL="https://drive.nvbes.fr"
export DRIVE_TOKEN="gx_test_replace_me"
export WORKSPACE_ID="00000000-0000-0000-0000-000000000001"
```

## Lister les Workspaces

```bash
curl -sS "$DRIVE_API_BASE_URL/v1/workspaces" \
  -H "Authorization: Bearer $DRIVE_TOKEN"
```

## Lister les Objets

```bash
curl -sS "$DRIVE_API_BASE_URL/v1/workspaces/$WORKSPACE_ID/objects?limit=50" \
  -H "Authorization: Bearer $DRIVE_TOKEN"
```

## Creer un Dossier

```bash
curl -sS "$DRIVE_API_BASE_URL/v1/workspaces/$WORKSPACE_ID/folders" \
  -H "Authorization: Bearer $DRIVE_TOKEN" \
  -H "Content-Type: application/json" \
  -H "Idempotency-Key: $(uuidgen)" \
  -d '{
    "parent_id": null,
    "name": "Clients"
  }'
```

## Creer un Upload

```bash
curl -sS "$DRIVE_API_BASE_URL/v1/workspaces/$WORKSPACE_ID/uploads" \
  -H "Authorization: Bearer $DRIVE_TOKEN" \
  -H "Content-Type: application/json" \
  -H "Idempotency-Key: $(uuidgen)" \
  -d '{
    "parent_id": null,
    "name": "contrat.pdf",
    "mime_type": "application/pdf",
    "expected_size_bytes": 1048576,
    "expected_checksum": "replace_with_sha256"
  }'
```

## Completer un Upload

```bash
export UPLOAD_ID="00000000-0000-0000-0000-000000000002"

curl -sS "$DRIVE_API_BASE_URL/v1/workspaces/$WORKSPACE_ID/uploads/$UPLOAD_ID/complete" \
  -H "Authorization: Bearer $DRIVE_TOKEN" \
  -H "Content-Type: application/json" \
  -H "Idempotency-Key: $(uuidgen)" \
  -d '{
    "size_bytes": 1048576,
    "checksum": "replace_with_sha256"
  }'
```

## Generer une Download URL

```bash
export OBJECT_ID="00000000-0000-0000-0000-000000000003"

curl -sS "$DRIVE_API_BASE_URL/v1/workspaces/$WORKSPACE_ID/objects/$OBJECT_ID/download-url" \
  -H "Authorization: Bearer $DRIVE_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{}'
```

## Creer un Lien Partage

```bash
curl -sS "$DRIVE_API_BASE_URL/v1/workspaces/$WORKSPACE_ID/objects/$OBJECT_ID/share-links" \
  -H "Authorization: Bearer $DRIVE_TOKEN" \
  -H "Content-Type: application/json" \
  -H "Idempotency-Key: $(uuidgen)" \
  -d '{
    "expires_at": "2026-06-12T12:00:00Z",
    "max_downloads": 5
  }'
```

## Lire le Quota

```bash
curl -sS "$DRIVE_API_BASE_URL/v1/workspaces/$WORKSPACE_ID/quota" \
  -H "Authorization: Bearer $DRIVE_TOKEN"
```
