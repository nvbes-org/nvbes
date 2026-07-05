# Local observability

La stack locale couvre Identity API, Identity web et Identity worker avec:

- Grafana: UI locale et provisioning declaratif.
- Prometheus: scrape local des endpoints `/metrics`.
- Mimir: remote-write local pour les metriques longues et k6.
- Tempo: traces OpenTelemetry.
- Loki: logs applicatifs et logs Faro.
- Pyroscope: profiles continus.
- Alloy: entree OTLP, Faro, Pyroscope et pipeline de redaction logs/traces.
- Beyla: eBPF opt-in pour instrumentation automatique Linux.
- k6: smoke de charge Identity avec remote-write Mimir.

## Demarrage

Depuis la racine:

```bash
pnpm dev:infra:obs
```

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

Pour envoyer Identity API et worker vers Alloy/Tempo/Pyroscope:

```bash
NVBES_OTLP_ENDPOINT=http://127.0.0.1:14317
NVBES_PROFILING_ENABLED=true
NVBES_PROFILING_ENDPOINT=http://127.0.0.1:4040
NVBES_IDENTITY_WORKER_METRICS_BIND_ADDR=127.0.0.1:4102
```

Pour activer Faro cote Identity web, apres consentement `grafana` ou
`performance`:

```bash
VITE_FARO_URL=http://localhost:12347/collect
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
- `grafana-dashboards/*.json` contient les dashboards versionnes.

Tempo est provisionne avec traces-to-logs vers Loki, traces-to-profiles vers
Pyroscope et traces-to-metrics vers Mimir. Le lien span -> profile ne devient
effectif que si l'application ajoute l'attribut `pyroscope.profile.id` aux spans
via un bridge OpenTelemetry compatible.

Les dashboards et alertes doivent rester limites a des labels techniques
(`job`, `status`, `method`, `path`, `outcome`, `queue`, `environment`) et ne
doivent jamais introduire email, tenant name, nom de fichier, object key,
payload ou contenu utilisateur.

## k6

Le smoke k6 cible `GET /health` sur Identity API et pousse ses metriques vers
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
IDENTITY_API_BASE_URL=http://host.docker.internal:4000 K6_VUS=2 K6_DURATION=1m \
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
