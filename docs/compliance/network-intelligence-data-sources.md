# Network Intelligence Data Sources

## MaxMind GeoLite2

GeoLite2 ASN, City, and Country are optional data sources. They must only be
enabled when `NVBES_MAXMIND_GEOLITE_EULA_ACCEPTED=true` and valid MaxMind
account credentials are configured.

Operational controls:

- Do not commit MaxMind archives, CSVs, MMDBs, account IDs, or license keys.
- Keep downloaded and cached MaxMind-derived records within the configured
  retention window.
- Use GeoLite2 for risk, fraud, routing, and residency decisions only where the
  product flow has a documented need.

## Loyalsoldier GeoIP

Loyalsoldier GeoIP is an optional network intelligence source backed by the
upstream project at https://github.com/Loyalsoldier/geoip.

Attribution:

- Source: Loyalsoldier/geoip
- Upstream: https://github.com/Loyalsoldier/geoip
- Licenses declared upstream: CC-BY-SA-4.0 and GPL-3.0
- Includes GeoLite2 data created by MaxMind, available from MaxMind.

Compliance controls:

- The source is disabled by default.
- Production use requires `NVBES_LOYALSOLDIER_GEOIP_ENABLED=true` and
  `NVBES_LOYALSOLDIER_GEOIP_LICENSE_ACCEPTED=true`.
- Do not bundle Loyalsoldier generated artifacts into proprietary distribution
  packages unless legal review confirms the distribution model and share-alike
  obligations are acceptable.
- Treat imported data as attributed third-party data in audit, support, and
  customer-facing disclosures when it materially affects a fraud, routing, or
  access decision.
- Keep the source code and source URL in `geo_ip_sources` so downstream exports
  can preserve attribution.

Commercial-use note:

The upstream license set is compatible with public/open-source redistribution
under the declared terms, but CC-BY-SA/GPL obligations can affect proprietary
or closed redistribution of derived datasets. For commercial hosted use, keep
the source opt-in, keep attribution, avoid redistributing raw generated data by
default, and require legal approval before packaging the data outside nvbes
controlled infrastructure.
