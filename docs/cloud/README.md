# nvbes Cloud Documentation

This directory is for nvbes Cloud customer-facing documentation.

It can describe managed hosting, billing, regions, support, SLA and cloud operations visible to customers.

## Runtime Taxonomy

Cloud documentation uses the target Account/Cloud taxonomy:

- `cloud-service` owns workspaces, Cloud resource policy, Drive resources,
  storage metadata, scans, quotas and Cloud public APIs.
- `cloud-web` owns the Cloud product surface for workspaces, Drive and completed
  Cloud product modules.
- `cloud-worker` owns Cloud async jobs: upload finalization, scans, cleanup,
  indexing and exports.
- `gateway-cloud` is the only Cloud gateway/BFF runtime name.

Do not introduce legacy runtime aliases in Cloud docs. Use the target names
above and the taxonomy recorded in
`docs/adr/2026-07-05-account-cloud-service-taxonomy.md`.

Account owns identity/session entry points. Billing owns billing source of
truth. Cloud may consume Account and Billing contracts, but Cloud documentation
must not describe direct database ownership across these contexts.
