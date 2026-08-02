# Runbook de smoke RGPD sur staging

## Objectif

Valider sur `staging` que les parcours RGPD `export` et `delete` fonctionnent de bout en bout:

- route API;
- création de privacy request;
- enfilement du job worker;
- traitement par le worker;
- envoi email si applicable;
- garde-fous de conformité.

## Pré-requis

- Un compte de test sur `staging` avec accès à sa boîte mail.
- Une session active sur ce compte.
- Un step-up récent si le parcours le demande.
- Un accès SQL en lecture sur la base `staging`.
- Les variables d'environnement suivantes:
  - `NVBES_STAGING_API_BASE_URL`
  - `NVBES_STAGING_DATABASE_URL`

## Parcours `POST /api/v1/auth/me/export`

### 1. Appeler la route

```bash
curl -i \
  -X POST \
  "$NVBES_STAGING_API_BASE_URL/api/v1/auth/me/export" \
  -H "Authorization: Bearer $ACCESS_TOKEN"
```

### 2. Attendus HTTP

- `200 OK`
- Réponse JSON avec `success: true`

### 3. Vérifications SQL

```sql
SELECT id, request_type, status, subject_user_id, requested_by, worker_job_id, requested_at
FROM privacy_requests
WHERE subject_user_id = '<USER_ID>'
ORDER BY requested_at DESC
LIMIT 5;
```

- Inspecter la queue Redis correspondante:

```bash
redis-cli --scan --pattern 'nvbes:worker_queue:privacy.account_export:job:*'
redis-cli --scan --pattern 'nvbes:worker_queue:email.send:job:*'
```

### 4. Attendus worker

- Le job `privacy.account_export` est consommé.
- Le job est marqué `succeeded`.
- Aucun `unknown job type` dans les logs.

### 5. Vérifications email

- L’email d’export est reçu par le bon destinataire.
- Le sujet mentionne la demande d’export de données.
- Le contenu confirme que la demande a bien été enregistrée.

## Parcours `POST /api/v1/closure`

### 1. Effectuer le step-up récent

Le compte doit avoir un step-up récent valide avant l’appel.

### 2. Appeler la route

```bash
curl -i \
  -X POST \
  "$NVBES_ACCOUNT_SERVICE_BASE_URL/api/v1/closure" \
  -H "Authorization: Bearer $ACCESS_TOKEN"
```

### 3. Attendus HTTP

- `202 Accepted`
- Réponse JSON avec `saga_id`, `status` et `requested_at`

### 4. Vérifications SQL

```sql
SELECT id, principal_id, status, requested_at, updated_at, completed_at, last_error
FROM account_closure_sagas
WHERE principal_id = '<USER_ID>'
ORDER BY requested_at DESC
LIMIT 1;
```

```sql
SELECT participant, ordinal, status, attempts, completed_at, last_error
FROM account_closure_participants
WHERE saga_id = '<SAGA_ID>'
ORDER BY ordinal;
```

### 5. Attendus worker

- Les checkpoints `cloud`, `billing`, `identity`, `account` passent à `completed` dans cet ordre.
- Les retries rejouent le même `event_id` sans répéter les effets distants.
- La saga et son événement outbox passent à `completed`/`published` seulement après la purge Account.

## Garde-fous à tester

### Rate limit export

- Réappeler `/api/v1/auth/me/export` jusqu’au seuil.
- Attendre un `429 Too Many Requests` après dépassement.

### Step-up manquant ou trop ancien

- Appeler `/api/v1/closure` sans scope `account:delete`.
- Attendre un refus d’autorisation.

### Compte avec workspaces possédés

- Si le compte possède encore des workspaces, appeler `/api/v1/closure` et suivre la saga.
- Attendre le checkpoint Cloud en échec avec `cloud_closure_conflict`.

### Workspace sous legal hold

- Pour le parcours workspace côté produit, appeler la suppression d’un workspace sous legal hold.
- Attendre un `409 Conflict`.

## Critères de succès

- Les jobs API, worker et email sont tous visibles et cohérents.
- Les statuts en base correspondent au résultat attendu.
- Les logs ne contiennent pas d’échec masqué.
- Les garde-fous de conformité répondent correctement.

## Notes d’exploitation

- Si le comportement du worker, du provider email ou des garde-fous change, mettre à jour ce runbook en même temps que le code.
- Si une suppression est rejouée, vérifier l’idempotence côté privacy request et côté queue Redis avant de valider le smoke.
