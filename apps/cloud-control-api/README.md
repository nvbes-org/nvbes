# Cloud Control API

Cloud-only control plane API boundary for managed nvbes workspaces.

The API owns provisioning requests, plan/apply orchestration, rollback requests
and operational status for managed cells. Domain rules live in
`libs/go/provisioning`; this app may only compose handlers, auth, routing and
transport concerns.

This area is Cloud-only and must not be exported to `nvbes-oss`.
