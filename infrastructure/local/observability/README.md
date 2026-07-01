# Local observability

Grafana local est provisionne declarativement:

- `grafana-datasources.yml` declare Prometheus, Tempo, Loki et Pyroscope.
- `grafana-provisioning/dashboards/nvbes.yml` charge les dashboards JSON.
- `grafana-provisioning/alerting/nvbes-critical.yml` charge les alertes critiques.
- `grafana-dashboards/*.json` contient les dashboards versionnes.

Tempo est provisionne avec traces-to-logs vers Loki et traces-to-profiles vers
Pyroscope. Le lien span -> profile ne devient effectif que si l'application
ajoute l'attribut `pyroscope.profile.id` aux spans via un bridge OpenTelemetry
compatible. Les dashboards et alertes doivent rester limites a des labels techniques
(`job`, `status`, `method`, `path`, `outcome`, `queue`, `environment`) et ne
doivent jamais introduire email, tenant name, nom de fichier, object key,
payload ou contenu utilisateur.

## Sentry et PostHog locaux

Sentry et PostHog ne font pas partie du profil local par defaut. Ils sont
geres comme stacks vendor optionnelles depuis le Dev Container avec:

```bash
pnpm dev:obs:sentry:install
pnpm dev:obs:sentry:up
pnpm dev:obs:posthog:install
pnpm dev:obs:posthog:up
```

Voir `docs/development/devcontainer.md` pour les garde-fous et les variables
locales a renseigner apres creation des projets/tokens.
