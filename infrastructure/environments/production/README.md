# Production

Ce dossier porte l'infrastructure et l'observabilite production. Le local garde
Prometheus, Tempo et Grafana OSS pour le developpement; la production envoie les
signaux vers Grafana Cloud via Grafana Alloy.

## Architecture observabilite

Flux production cible:

```text
nvbes APIs /metrics ──────┐
nvbes workers /metrics ───┤─ Grafana Alloy ── remote_write ── Grafana Cloud Metrics
nvbes OTLP traces ────────┤                 ├─ OTLP HTTP ──── Grafana Cloud Traces + PostHog Traces
nvbes app logs ───────────┤                 ├─ Loki push ──── Grafana Cloud Logs
                           │                 ├─ OTLP HTTP ──── PostHog Logs
nvbes Pyroscope profiles ─┘                 └─ Pyroscope ──── Grafana Cloud Profiles
```

Alloy est obligatoire entre les services et Grafana Cloud. Les SDK applicatifs
ne doivent pas porter de credentials Grafana Cloud en production. Alloy fournit
un point local stable, evite l'exposition publique des endpoints
d'observabilite, centralise l'authentification, l'etiquetage, la redaction, le
sampling et le routage vers Metrics/Mimir, Logs/Loki, Traces/Tempo et
Profiles/Pyroscope.

## Fichiers

- `backend.tf`, `backend.hcl.example`: état Terraform distant sur Object
  Storage Scaleway, avec verrouillage natif S3.
- `main.tf`: réseau privé, base privée, origine Cloudflare-only, WAF managé,
  egress contrôlé et archive d’audit WORM externe.
- `../../stacks/email/production`: stack produit email indépendante, avec son
  propre état Terraform, son DNS/TEM et son observabilité.
- `siem.tf`: import des six règles Loki dans Grafana Cloud et routage vers le
  contact point Security on-call.
- `variables.tf`, `outputs.tf`, `terraform.tfvars.example`: contrat de
  déploiement sans valeur réelle.
- `docker-compose.observability.yml`: service Alloy production.
- `alloy.config.alloy`: pipeline metrics, traces, logs et profiles vers
  Grafana Cloud, avec dual-export traces/logs vers PostHog.
- `observability.env.example`: variables requises, sans secret reel.

## Stack email séparée

L'infrastructure email est gérée exclusivement depuis
`infrastructure/stacks/email/production`. La stack générale ne possède ni le
domaine TEM, ni ses DNS, ni SNS, ni les secrets et dashboards du worker email.
Consulter le README de cette stack avant tout plan afin de préserver la
séparation des états.

## Secrets requis

Ne pas committer ces valeurs. Elles doivent venir du secret manager de
l'environnement.

Le job d’ancrage référence deux secrets Scaleway protégés et indépendants:

- `audit_anchor_kms_auth_token_secret_id`: secret de l’identité du compte
  production, limitée à la signature et à la vérification KMS;
- `audit_archive_writer_secret_id`: secret de l’identité du compte Security,
  limitée à `PutObject` sous `anchors/`, sans lecture ni suppression.

Ces secrets sont injectés respectivement dans
`NVBES_AUDIT_ANCHOR_KMS_AUTH_TOKEN` et
`NVBES_AUDIT_ANCHOR_S3_SECRET_KEY`; leurs valeurs n’apparaissent pas dans la
définition du job. Les access keys non secrètes sont injectées séparément. Le
job signe toutes les 15 minutes le digest des têtes de chaîne avec Key Manager,
vérifie la signature, écrit le manifeste dans le bucket Object Lock
`COMPLIANCE`, puis enregistre le reçu local append-only.

Le bucket et son identité d’écriture sont créés par le stack Terraform
`../security-audit-archive`, avec un backend d’état et des credentials auxquels
le compte production n’a pas accès. L’équipe Security transmet uniquement le
nom du bucket, l’access key d’écriture et place la secret key dans le secret
protégé référencé par `audit_archive_writer_secret_id`. La production ne doit
jamais recevoir les credentials Terraform du compte Security. L’accès humain à
ce compte suit une procédure JIT avec double contrôle.

```bash
GRAFANA_CLOUD_PROMETHEUS_URL="https://prometheus-<region>.grafana.net/api/prom/push"
GRAFANA_CLOUD_PROMETHEUS_USER="<stack-prometheus-user>"
GRAFANA_CLOUD_PROMETHEUS_TOKEN_FILE="/etc/nvbes/secrets/grafana_cloud_prometheus_token"

GRAFANA_CLOUD_OTLP_ENDPOINT="https://otlp-gateway-<region>.grafana.net/otlp"
GRAFANA_CLOUD_OTLP_USER="<stack-otlp-user>"
GRAFANA_CLOUD_OTLP_TOKEN_FILE="/etc/nvbes/secrets/grafana_cloud_otlp_token"

GRAFANA_CLOUD_LOKI_URL="https://logs-prod-<region>.grafana.net/loki/api/v1/push"
GRAFANA_CLOUD_LOKI_USER="<stack-loki-user>"
GRAFANA_CLOUD_LOKI_TOKEN_FILE="/etc/nvbes/secrets/grafana_cloud_loki_token"

GRAFANA_CLOUD_PROFILES_URL="https://profiles-prod-<region>.grafana.net"
GRAFANA_CLOUD_PROFILES_USER="<stack-profiles-user>"
GRAFANA_CLOUD_PROFILES_TOKEN_FILE="/etc/nvbes/secrets/grafana_cloud_profiles_token"

POSTHOG_PROJECT_TOKEN_FILE="/etc/nvbes/secrets/posthog_project_token"
POSTHOG_OTLP_HOST="https://eu.i.posthog.com"
POSTHOG_OTLP_LOGS_ENDPOINT="https://eu.i.posthog.com/i/v1/logs"
POSTHOG_OTLP_TRACES_ENDPOINT="https://eu.i.posthog.com/i/v1/traces"

NVBES_LOGS_DIR="/var/log/nvbes"
NVBES_OBSERVABILITY_INTERNAL_TOKEN_FILE="/etc/nvbes/secrets/nvbes_observability_internal_token"
```

Les fichiers `*_TOKEN_FILE` contiennent uniquement le token brut correspondant.
`POSTHOG_PROJECT_TOKEN_FILE` contient le project token PostHog utilise comme
Bearer token OTLP. Utiliser PostHog EU Cloud ou un proxy first-party; ne pas
pointer vers un endpoint US direct.

## Demarrage

1. Creer les secrets sur l'hote ou dans l'orchestrateur.
2. Copier `observability.env.example` vers un fichier d'environnement hors repo.
3. Renseigner les URLs Grafana Cloud et les targets internes nvbes.
4. Demarrer Alloy:

```bash
docker compose \
  --env-file /etc/nvbes/observability.env \
  -f infrastructure/environments/production/docker-compose.observability.yml \
  up -d
```

5. Configurer les services Rust avec:

```bash
NVBES_OTLP_ENDPOINT=http://127.0.0.1:4317
NVBES_OTLP_AUTHORIZATION_HEADER=
NVBES_PROFILING_ENABLED=true
NVBES_PROFILING_ENDPOINT=http://127.0.0.1:4040
NVBES_PROFILING_SAMPLE_RATE_HZ=100
NVBES_PROFILING_BASIC_AUTH_USER=
NVBES_PROFILING_BASIC_AUTH_PASSWORD=
NVBES_OBSERVABILITY_INTERNAL_TOKEN=<meme secret que le fichier Alloy>
NVBES_CLOUD_WORKER_METRICS_BIND_ADDR=127.0.0.1:4101
NVBES_ACCOUNT_WORKER_METRICS_BIND_ADDR=127.0.0.1:4102
```

Garder les binds workers sur `127.0.0.1` quand Alloy tourne en sidecar ou sur
le meme hote. Si Alloy scrape via DNS interne (`*.internal.nvbes.fr`), binder
les workers sur une interface reseau privee uniquement, jamais sur une interface
publique.

`NVBES_OTLP_AUTHORIZATION_HEADER` reste vide quand les services exportent vers
Alloy en local. En production, la configuration Rust refuse cette variable pour
empecher un export OTLP direct vers Grafana Cloud.

`NVBES_PROFILING_*` utilise le SDK Pyroscope Rust avec backend pprof-rs. Le
profiling est opt-in (`NVBES_PROFILING_ENABLED=true`) et doit pointer vers
Alloy en local. En production, la configuration Rust refuse
`NVBES_PROFILING_BASIC_AUTH_*`; les credentials Profiles restent montes
uniquement dans Alloy.

## Privacy-by-design

- `/metrics` reste interne et protege par `Authorization: Bearer`.
- Alloy n'expose OTLP, Pyroscope et son UI technique que sur `127.0.0.1` par
  defaut.
- Les labels metrics doivent rester techniques: `service`, `environment`,
  `method`, `path_template`, `status`, `outcome`.
- Interdit dans metrics/traces/logs: email, nom de fichier, object key, token,
  signed URL, payload utilisateur, contenu de fichier.
- Les traces passent par `otelcol.processor.attributes` puis
  `otelcol.processor.tail_sampling`: suppression IP/user-agent/object key et
  conservation prioritaire des erreurs/lenteurs avec baseline sample 20% avant
  export Grafana et PostHog.
- Les logs applicatifs lus depuis `NVBES_LOGS_DIR` passent par `loki.process`:
  redaction secrets/email/IP/champs sensibles et drop des lignes > 16KB.
  Aucun sampling n’est appliqué aux journaux sécurité utilisés par le SIEM.
- Le flux PostHog Logs relit les memes fichiers via `otelcol.receiver.filelog`
  et applique une redaction OTLP dediee avant `POSTHOG_OTLP_LOGS_ENDPOINT`.
  Alloy doit etre lance avec `--stability.level=public-preview` pour ce
  receiver. Ne pas exporter les metrics vers PostHog dans cette passe.
- Le profiling continu exporte uniquement des stacks CPU et des labels
  techniques (`service`, `environment`, `platform`). Ne jamais ajouter de tags
  dynamiques issus d'un tenant, utilisateur, chemin fichier, object key ou
  payload.
- Aucun endpoint pprof HTTP applicatif n'est expose. Les services poussent vers
  Alloy; Alloy seul pousse vers Grafana Cloud Profiles avec secret monte.

## Verification

```bash
curl -fsS http://127.0.0.1:12345/-/ready
curl -fsS -H "Authorization: Bearer $NVBES_OBSERVABILITY_INTERNAL_TOKEN" \
  "$NVBES_ACCOUNT_SERVICE_METRICS_TARGET/metrics" | head
curl -fsS -H "Authorization: Bearer $NVBES_OBSERVABILITY_INTERNAL_TOKEN" \
  "$NVBES_CLOUD_WORKER_METRICS_TARGET/metrics" | head
curl -fsS -H "Authorization: Bearer $NVBES_OBSERVABILITY_INTERNAL_TOKEN" \
  "$NVBES_EMAIL_WORKER_METRICS_TARGET/metrics" | head
```

Dans Grafana Cloud:

- Metrics APIs: `http_requests_total{environment="production"}`
- Metrics workers: `worker_queue_jobs_total{environment="production"}`
- Metrics email: `email_messages_total{service="email-worker",environment="production"}`
- Traces: service `nvbes-account-service` ou `nvbes-cloud-service`
- PostHog Traces: evenement trace visible dans PostHog uniquement apres
  redaction et tail sampling Alloy.
- PostHog Logs: lignes JSON redacted, sans email/IP/token/object key.
- Profiles: applications `account-service`, `cloud-service`, `account-worker`,
  `cloud-worker`
- Logs: `{platform="nvbes", environment="production"}`

## Traces to profiles

Grafana affiche le lien d'un span vers un profil seulement si trois conditions
sont reunies: traces OTLP vers Tempo, profils Pyroscope vers Profiles, et bridge
OpenTelemetry ajoutant `pyroscope.profile.id` sur les spans. Le repo provisionne
Tempo/Pyroscope et le pipeline de profils; l'attribut span-profile reste a
ajouter lorsque le SDK Rust expose un bridge stable compatible.

## References

- Grafana recommande Alloy pour l'architecture production OTLP afin d'ajouter
  fiabilite, enrichissement, sampling, redaction et routage.
- Grafana documente `tracesToProfiles` comme configuration Tempo -> Pyroscope,
  mais exige aussi un bridge span-profiles cote application.
- Le local `infrastructure/local/docker-compose.yml` reste volontairement limite
  a Prometheus, Tempo, Loki, Pyroscope et Grafana OSS.
