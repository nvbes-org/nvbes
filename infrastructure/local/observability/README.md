# Local observability

Grafana local est provisionne declarativement:

- `grafana-datasources.yml` declare Prometheus et Tempo uniquement.
- `grafana-provisioning/dashboards/nvbes.yml` charge les dashboards JSON.
- `grafana-provisioning/alerting/nvbes-critical.yml` charge les alertes critiques.
- `grafana-dashboards/*.json` contient les dashboards versionnes.

Le datasource Tempo ne reference pas Loki tant que Loki n'est pas deploye. Les
dashboards et alertes doivent rester limites a des labels techniques
(`job`, `status`, `method`, `path`, `outcome`, `queue`, `environment`) et ne
doivent jamais introduire email, tenant name, nom de fichier, object key,
payload ou contenu utilisateur.
