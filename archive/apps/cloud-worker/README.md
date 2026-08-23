# Cloud Worker

Cloud worker owns Cloud async jobs: upload finalization, scans, cleanup,
indexing and export processing.

It consumes provider-neutral Cloud job contracts and must not embed
provider-specific infrastructure apply logic.

Cloud app boundary names are `cloud-service`, `cloud-web`, `cloud-worker` and
`gateway-cloud`.
