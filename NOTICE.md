# nvbes Notices

This product can optionally use third-party network intelligence datasets for
GeoIP, routing, fraud scoring, and access-risk decisions. These datasets are
disabled by default and must not be committed to this repository.

## MaxMind GeoLite2

Optional GeoLite2 ASN, City, and Country data is created by MaxMind and is
available from MaxMind. Use requires explicit acceptance of the MaxMind GeoLite2
EULA through `NVBES_MAXMIND_GEOLITE_EULA_ACCEPTED=true` and valid MaxMind
account credentials.

MaxMind-derived downloaded or cached records must stay within the configured
retention window and must not be redistributed from this repository.

## Loyalsoldier GeoIP

Optional Loyalsoldier GeoIP data is sourced from:

- Project: Loyalsoldier/geoip
- Upstream: https://github.com/Loyalsoldier/geoip
- Declared upstream licenses: CC-BY-SA-4.0 and GPL-3.0
- Includes GeoLite2 data created by MaxMind, available from MaxMind.

Production use requires `NVBES_LOYALSOLDIER_GEOIP_ENABLED=true` and
`NVBES_LOYALSOLDIER_GEOIP_LICENSE_ACCEPTED=true`.

Do not bundle Loyalsoldier generated artifacts into proprietary distribution
packages unless legal review confirms the distribution model and share-alike
obligations are acceptable.
