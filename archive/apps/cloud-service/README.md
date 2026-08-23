# Cloud Service

Cloud service owns the Cloud API runtime for workspaces, Drive resources,
storage metadata, scans, quotas and Cloud public APIs.

It consumes Account identity/session contracts and Billing entitlement
contracts, but it does not own Account or Billing source-of-truth tables.

Provisioning plan/apply orchestration stays provider-neutral at the service
boundary; provider-specific infrastructure belongs under `deploy/cloud`.
