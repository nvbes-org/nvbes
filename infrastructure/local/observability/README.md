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
