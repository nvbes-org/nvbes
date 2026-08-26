# API Publique V1 - Erreurs et Rate Limits

> **Statut : annexe d'une future API produit Cloud/Drive, hors V1 active.**

## Format d'Erreur

Toutes les erreurs publiques V1 utilisent le meme wrapper.

```json
{
  "error": {
    "code": "quota_exceeded",
    "message": "Le quota de stockage de cet espace est atteint.",
    "requestId": "req_..."
  }
}
```

Regles:

- `code` est stable dans `/v1`;
- `message` peut etre clarifie sans changement majeur;
- `requestId` doit permettre la correlation support;
- les details internes, object keys, secrets et traces SQL ne sont jamais exposes.

## Codes Stables

| Code | HTTP | Sens |
| --- | ---: | --- |
| `invalid_api_key` | 401 | Credential absent ou invalide. |
| `revoked_api_key` | 401 | Cle revoquee. |
| `expired_api_key` | 401 | Cle ou token expire. |
| `insufficient_scope` | 403 | Scope manquant. |
| `workspace_suspended` | 403 | Workspace bloque par policy ou billing. |
| `object_not_found` | 404 | Objet absent ou inaccessible. |
| `share_link_not_found` | 404 | Lien absent ou inaccessible. |
| `upload_expired` | 410 | Session d'upload expiree. |
| `validation_failed` | 422 | Payload invalide. |
| `quota_exceeded` | 409 | Quota insuffisant. |
| `idempotency_conflict` | 409 | Meme cle idempotente avec payload different. |
| `rate_limited` | 429 | Limite atteinte. |
| `internal_error` | 500 | Erreur interne generique. |

## Rate Limits par Plan

| Plan | Requetes/min | Requetes/jour | API keys legacy |
| --- | ---: | ---: | ---: |
| Solo Pro | 60 | 1 000 | 1 |
| Team | 300 | 20 000 | 5 |
| Team Plus | 600 | 100 000 | 20 |

Limites additionnelles:

- upload sessions par heure;
- download URLs par minute;
- share links crees par jour;
- egress mensuel;
- erreurs d'authentification par credential et par IP.

## Headers de Limite

Les reponses rate limitees doivent retourner:

```http
HTTP/1.1 429 Too Many Requests
Retry-After: 60
X-RateLimit-Limit: 60
X-RateLimit-Remaining: 0
X-RateLimit-Reset: 1780660860
```

Pour les reponses non limitees, les headers `X-RateLimit-*` sont recommandes mais pas requis pour la V1 initiale.

## Idempotence

Header:

```http
Idempotency-Key: <client-generated-key>
```

Operations concernees:

- creation de dossier;
- creation d'upload session;
- completion upload;
- creation lien partage.

Une cle idempotente rejouee avec un payload different retourne `idempotency_conflict`.
