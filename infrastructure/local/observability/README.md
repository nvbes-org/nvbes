# Local observability

La stack locale couvre Account Service, Account web et Account worker avec:

- Grafana: UI locale et provisioning declaratif.
- Prometheus: scrape local des endpoints `/metrics`.
- Mimir: remote-write local pour les metriques longues et k6.
- Tempo: traces OpenTelemetry.
- Loki: logs applicatifs et logs Faro.
- Pyroscope: profiles continus.
- Alloy: entree OTLP, Faro, Pyroscope et pipeline de redaction logs/traces.
- Beyla: eBPF opt-in pour instrumentation automatique Linux.
- k6: smoke de charge Account avec remote-write Mimir.

## Demarrage

Depuis la racine:

```bash
pnpm dev:infra:obs
```

Pour demarrer en une commande l'infrastructure locale, Account Service,
Account worker et Account web avec tous les exports actifs:

```bash
pnpm dev:account:observability
```

Cette commande configure OTLP, Pyroscope, Faro, les metriques du worker et les
logs JSON. Les logs backend sont ecrits dans `app-logs/account-service.jsonl`
et `app-logs/account-worker.jsonl`, puis lus et expurges par Alloy avant Loki.

Pour tester le meme frontend local contre l'application Frontend
Observability `account-web-dev` de Grafana Cloud:

```bash
pnpm dev:account:observability:cloud
```

La commande utilise `VITE_FARO_URL_ACCOUNT_WEB` lorsqu'elle est deja definie.
Sinon, elle lit la variable GitHub `GRAFANA_FARO_URL_ACCOUNT_WEB` avec `gh`.
Elle envoie uniquement Faro vers Grafana Cloud avec
`deployment.environment=development`; elle ne demarre pas un second pipeline
Faro local.

Endpoints locaux principaux:

- Grafana: `http://localhost:3000`
- Alloy UI: `http://localhost:12345`
- Faro receiver: `http://localhost:12347/collect`
- OTLP gRPC: `http://127.0.0.1:14317`
- OTLP HTTP: `http://127.0.0.1:14318`
- Pyroscope ingest via Alloy: `http://127.0.0.1:4040`
- Pyroscope UI/API direct: `http://localhost:4041`
- Mimir: `http://localhost:9009`
- Prometheus: `http://localhost:9090`
- Tempo: `http://localhost:3200`
- Loki: `http://localhost:3100`

## Variables applicatives

Pour envoyer Account Service et worker vers Alloy/Tempo/Pyroscope:

```bash
NVBES_OTLP_ENDPOINT=http://127.0.0.1:14317
NVBES_PROFILING_ENABLED=true
NVBES_PROFILING_ENDPOINT=http://127.0.0.1:4040
NVBES_ACCOUNT_WORKER_METRICS_BIND_ADDR=127.0.0.1:4102
```

Pour activer Faro cote Account web, apres consentement `grafana` ou
`performance`:

```bash
VITE_FARO_URL_ACCOUNT_WEB=http://localhost:12347/collect
VITE_FARO_ENVIRONMENT=development
VITE_FARO_TRACING_ORIGINS=http://localhost:4000,http://localhost:3001
VITE_FARO_SESSION_SAMPLE_RATE=1
```

Les valeurs vides gardent la collecte inactive. Ne pas renseigner de token ou
secret reel dans `.env.example`.

## Grafana

Grafana est provisionne declarativement:

- `grafana-datasources.yml` declare Prometheus, Mimir, Tempo, Loki et Pyroscope.
- `grafana-provisioning/dashboards/nvbes.yml` charge les dashboards JSON.
- `grafana-provisioning/alerting/nvbes-critical.yml` charge les alertes critiques.
- `grafana-provisioning/alerting/nvbes-account.yml` surveille le heartbeat et
  l'age des files Account.
- `grafana-dashboards/*.json` contient les dashboards versionnes.
- `nvbes-account-observability.json` regroupe les golden signals Account web,
  service et worker.

Tempo est provisionne avec traces-to-logs vers Loki, traces-to-profiles vers
Pyroscope et traces-to-metrics vers Mimir. Le lien span -> profile ne devient
effectif que si l'application ajoute l'attribut `pyroscope.profile.id` aux spans
via un bridge OpenTelemetry compatible.

Les dashboards et alertes doivent rester limites a des labels techniques
(`job`, `status`, `method`, `path`, `outcome`, `queue`, `environment`) et ne
doivent jamais introduire email, tenant name, nom de fichier, object key,
payload ou contenu utilisateur.

## Verification

Une fois les trois runtimes demarres:

```bash
curl --fail http://localhost:4000/health
curl --fail http://localhost:4000/metrics
curl --fail http://localhost:4102/metrics
curl --fail http://localhost:3040/metrics
curl --fail http://localhost:12345/-/ready
```

Dans Grafana, ouvrir les dashboards `nvbes Account Observability` et
`nvbes Email and Communications`. Les requetes
HTTP doivent apparaitre dans Prometheus, les traces dans Tempo, les logs
backend et Faro dans Loki, et les profils `account-service`/`account-worker`
dans Pyroscope.

## k6

Le smoke k6 cible `GET /health` sur Account Service et pousse ses metriques vers
Mimir:

```bash
docker compose --profile loadtest -f infrastructure/local/docker-compose.yml run --rm k6-identity-smoke
```

Equivalent pnpm:

```bash
pnpm dev:infra:obs:k6:identity
```

Surcharges utiles:

```bash
ACCOUNT_SERVICE_BASE_URL=http://host.docker.internal:4000 K6_VUS=2 K6_DURATION=1m \
  docker compose --profile loadtest -f infrastructure/local/docker-compose.yml run --rm k6-identity-smoke
```

## Beyla

Beyla est opt-in parce que l'eBPF exige Linux et des privileges eleves. Lancer
la stack standard puis:

```bash
docker compose --profile observability --profile beyla -f infrastructure/local/docker-compose.yml up alloy-beyla
```

Equivalent pnpm:

```bash
pnpm dev:infra:obs:beyla
```

Le profil instrumente les ports Identity `4000` et `4102`, pousse les metriques
vers Mimir et les traces vers Alloy OTLP.

## Sentry et PostHog

Le Dev Container ne demarre plus de services locaux Sentry ou PostHog. Pour
activer la capture applicative, renseigner dans `.env` des DSN/tokens pointant
vers des deploiements externes. Les valeurs vides gardent la capture inactive.
