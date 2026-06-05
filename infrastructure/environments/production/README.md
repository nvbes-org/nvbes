# Production

Ce dossier porte la configuration d'observabilite production. Le local garde
Prometheus, Tempo et Grafana OSS pour le developpement; la production envoie les
signaux vers Grafana Cloud via Grafana Alloy.

## Architecture observabilite

Flux production cible:

```text
nvbes APIs /metrics ──────┐
nvbes workers /metrics ───┤─ Grafana Alloy ── remote_write ── Grafana Cloud Metrics
nvbes OTLP traces ────────┘                 └─ OTLP HTTP ──── Grafana Cloud Traces
```

Alloy est obligatoire entre les services et Grafana Cloud. Il fournit un point
local stable, evite l'exposition publique des endpoints d'observabilite, et
centralise l'authentification, l'etiquetage et les futures regles de redaction.

## Fichiers

- `docker-compose.observability.yml`: service Alloy production.
- `alloy.config.alloy`: pipeline metrics + traces vers Grafana Cloud.
- `observability.env.example`: variables requises, sans secret reel.

## Secrets requis

Ne pas committer ces valeurs. Elles doivent venir du secret manager de
l'environnement.

```bash
GRAFANA_CLOUD_PROMETHEUS_URL="https://prometheus-<region>.grafana.net/api/prom/push"
GRAFANA_CLOUD_PROMETHEUS_USER="<stack-prometheus-user>"
GRAFANA_CLOUD_PROMETHEUS_TOKEN_FILE="/etc/nvbes/secrets/grafana_cloud_prometheus_token"

GRAFANA_CLOUD_OTLP_ENDPOINT="https://otlp-gateway-<region>.grafana.net/otlp"
GRAFANA_CLOUD_OTLP_USER="<stack-otlp-user>"
GRAFANA_CLOUD_OTLP_TOKEN_FILE="/etc/nvbes/secrets/grafana_cloud_otlp_token"

NVBES_OBSERVABILITY_INTERNAL_TOKEN_FILE="/etc/nvbes/secrets/nvbes_observability_internal_token"
```

Les fichiers `*_TOKEN_FILE` contiennent uniquement le token brut correspondant.

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
NVBES_OBSERVABILITY_INTERNAL_TOKEN=<meme secret que le fichier Alloy>
NVBES_DRIVE_WORKER_METRICS_BIND_ADDR=127.0.0.1:4101
NVBES_IDENTITY_WORKER_METRICS_BIND_ADDR=127.0.0.1:4102
```

Garder les binds workers sur `127.0.0.1` quand Alloy tourne en sidecar ou sur
le meme hote. Si Alloy scrape via DNS interne (`*.internal.nvbes.fr`), binder
les workers sur une interface reseau privee uniquement, jamais sur une interface
publique.

`NVBES_OTLP_AUTHORIZATION_HEADER` reste vide quand les services exportent vers
Alloy en local. Ne le renseigner que pour un export OTLP direct vers Grafana
Cloud, par exemple `Basic <base64(instance_id:token)>`.

## Privacy-by-design

- `/metrics` reste interne et protege par `Authorization: Bearer`.
- Alloy n'expose OTLP que sur `127.0.0.1` par defaut.
- Les labels metrics doivent rester techniques: `service`, `environment`,
  `method`, `path_template`, `status`, `outcome`.
- Interdit dans metrics/traces/logs: email, nom de fichier, object key, token,
  signed URL, payload utilisateur, contenu de fichier.
- Les logs applicatifs ne sont pas encore envoyes a Grafana Cloud. Ajouter Loki
  seulement apres une passe de redaction dediee.
- Le profiling continu n'est pas active. Ajouter Pyroscope seulement apres
  instrumentation explicite et revue privacy.

## Verification

```bash
curl -fsS http://127.0.0.1:12345/-/ready
curl -fsS -H "Authorization: Bearer $NVBES_OBSERVABILITY_INTERNAL_TOKEN" \
  "$NVBES_IDENTITY_API_METRICS_TARGET/metrics" | head
curl -fsS -H "Authorization: Bearer $NVBES_OBSERVABILITY_INTERNAL_TOKEN" \
  "$NVBES_DRIVE_WORKER_METRICS_TARGET/metrics" | head
```

Dans Grafana Cloud:

- Metrics APIs: `http_requests_total{environment="production"}`
- Metrics workers: `worker_queue_jobs_total{environment="production"}`
- Traces: service `nvbes-identity-api` ou `nvbes-drive-api`

## References

- Grafana recommande Alloy pour l'architecture production OTLP afin d'ajouter
  fiabilite, enrichissement, sampling, redaction et routage.
- Le local `infrastructure/local/docker-compose.yml` reste volontairement limite
  a Prometheus, Tempo et Grafana OSS.
