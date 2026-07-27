# Runbooks Incidents - Beta

## Cadre

Severites:

- `SEV1`: fuite de donnees, bucket public, corruption/perte donnees, indisponibilite majeure.
- `SEV2`: parcours critique indisponible, migration staging cassee, billing webhook bloque, worker RGPD bloque.
- `SEV3`: degradation limitee, alerte non critique, bug avec contournement.

Pour chaque incident:

1. Designer un incident owner.
2. Ouvrir un canal incident.
3. Noter heure de debut, impact, environnement, release courante, `request_id` si disponible.
4. Preserver logs et audit.
5. Contenir avant de corriger.
6. Documenter decision, mitigation, verification et postmortem si SEV1/SEV2.

## API Down

Signaux:

- `/health` en erreur ou timeout.
- hausse 5xx;
- alert `api-5xx-high`;
- smoke staging rouge.

Diagnostic:

```bash
curl -i "$NVBES_STAGING_API_BASE_URL/health"
curl -i "$NVBES_STAGING_API_BASE_URL/metrics"
```

Actions:

- Verifier dernier deploiement API.
- Verifier variables `NVBES_ENV`, `NVBES_DATABASE_URL`, `NVBES_API_PORT`.
- Verifier connectivite PostgreSQL.
- Redemarrer API si crashloop.
- Rollback binaire si incident post-release.

Validation:

- `/health` ok.
- `pnpm test:smoke:staging` vert.
- 5xx stabilises.

## PostgreSQL Down ou Saturation

Signaux:

- `/health` timeout;
- logs `database_error`;
- pool sature;
- alerte `postgres-saturation`.

Actions:

- Confirmer etat Scaleway Managed PostgreSQL.
- Verifier endpoint prive et firewall.
- Suspendre migrations en cours.
- Reduire trafic beta si saturation.
- Restaurer depuis backup uniquement avec validation privacy si corruption.

Validation:

- connexions API retablies;
- migrations non bloquees;
- `pnpm test:smoke:staging` vert.

## Object Storage Indisponible

Signaux:

- echecs upload/download;
- erreurs storage dans dashboard `uploads-downloads`;
- hausse tickets beta sur partage public.

Actions:

- Verifier statut Scaleway Object Storage.
- Verifier credentials runtime.
- Verifier CORS bucket et endpoint.
- Desactiver temporairement creation de liens publics si exposition non fiable.
- Ne jamais rendre le bucket public pour contourner l'incident.

Validation:

- upload session creee;
- download URL creee;
- lien public test repond sans 5xx.

## Bucket Public Suspect

Signaux:

- alert `public-bucket-suspected`;
- scan policy detecte ACL publique;
- acces direct non signe possible.

Severite: SEV1.

Actions immediates:

- Bloquer tout acces public bucket.
- Revoquer credentials suspects.
- Preserver logs object storage, Cloudflare et API.
- Identifier fenetre d'exposition.
- Ouvrir analyse violation donnees personnelles si contenu client potentiellement expose.

Validation:

- listing public impossible;
- objet direct impossible sans URL signee valide;
- nouvelles URLs signees courtes fonctionnent.

## Billing Webhook Failure

Signaux:

- `billing_webhook_events.status = failed`;
- Stripe retries;
- droits produit incoherents;
- checkout complete mais subscription non projetee.

Actions:

- Verifier `NVBES_STRIPE_WEBHOOK_SECRET`.
- Verifier horodatage/signature.
- Identifier `provider_event_id`.
- Rejouer uniquement si idempotence confirmee.
- Ne pas modifier manuellement plan/quota sans audit.

Validation:

- webhook passe `processed`;
- subscription interne a jour;
- audit `billing.updated` present.

## Public API Network Risk

Signaux:

- alerte `nvbes-public-api-network-policy-blocks`;
- alerte `nvbes-public-api-high-geo-risk-ratio`;
- hausse de `drive_public_api_network_policy_blocks_total`;
- hausse du ratio `drive_public_api_high_risk_requests_total / drive_public_api_geo_requests_total`;
- logs/audit `network_risk_blocked` avec `network_block_reason`.

Severite: SEV2 si un workspace legitime est bloque, SEV3 si c'est un pic attendu de trafic hostile.

Actions:

- Identifier `reason`, `mode`, `workspace_id`, `request_id` et `geo_network_kind`.
- Verifier si la policy workspace est en `enforce`, `monitor_only` ou `disabled`.
- Comparer les labels `vpn`, `proxy`, `tor`, `datacenter` et le `geo_risk_score`.
- Contacter le workspace owner si le blocage touche un trafic legitime.
- Ajouter une allowlist CIDR expiree uniquement si l'origine est verifiee.
- Passer temporairement en `monitor_only` si un faux positif large bloque une integration critique.

Validation:

- le compteur de blocs se stabilise;
- les requetes legitimes repassent sans 403;
- les audits conservent `metadata.geo` et `network_block_reason`;
- l'allowlist temporaire a une date d'expiration.

## Worker Jobs

Signaux:

- `Account worker heartbeat stale`;
- `Account worker queue is stale`;
- `up{job="account-worker"} == 0`;
- hausse de `worker_queue_jobs_total` avec `outcome=~"retry_scheduled|dead_letter"`.

Actions:

- Verifier le statut du worker, puis ses logs correles par `trace_id` et `job.type`.
- Inspecter `worker_queue_depth` et `worker_queue_oldest_age_seconds` par queue et statut.
- Verifier PostgreSQL et Redis avant de redemarrer le worker.
- Identifier la cause du dernier echec avant tout reenqueuing.
- Reenqueuer une dead letter seulement avec la meme cle d'idempotence et apres correction.

Validation:

- `up{job="account-worker"} == 1`;
- le heartbeat a moins de sept minutes;
- l'age de la plus vieille entree diminue jusqu'a zero;
- les nouveaux jobs terminent avec `outcome="success"`;
- aucun doublon metier n'a ete cree.

## Jobs RGPD Bloques

Signaux:

- jobs `privacy.*` en `failed` ou `dead_letter`;
- demande RGPD reste `queued` ou `processing`;
- alerte jobs critiques.

Actions:

- Inspecter la queue Redis `privacy.*` et `privacy_requests`.
- Relancer worker `worker run-once` en staging si le bug est transitoire.
- Corriger puis reenqueuer avec meme idempotency key si possible.
- Si deadline RGPD menacee, escalader security/privacy owner.

Validation:

- job `succeeded`;
- privacy request `completed` ou `rejected` avec raison documentee;
- audit workspace/user present.

## Migration Staging Echouee

Signaux:

- `pnpm db:migrate:staging` echoue;
- API ne demarre plus apres migration;
- schema incoherent.

Actions:

- Stopper deploiement.
- Capturer erreur migration et version appliquee.
- Ne pas relancer a l'aveugle.
- Si aucune donnee beta critique: restaurer snapshot staging.
- Si donnees beta a conserver: preparer migration corrective forward-only.

Validation:

- `cargo run -p nvbes-cloud-service -- migrate` termine;
- API demarre;
- smoke staging vert.

## Rollback Applicatif

Declencheurs:

- regression P0/P1 sur auth, upload, partage public, audit, billing ou privacy;
- smoke staging rouge apres deploiement;
- hausse 5xx persistante.

Actions:

- Revenir au dernier artefact API/web connu stable.
- Ne pas rollback DB destructivement.
- Si migration incompatible, appliquer correctif forward-only.
- Relancer smoke.

Validation:

- parcours critique retabli;
- logs sans nouvelle erreur critique;
- incident cloture avec cause et action preventive.
