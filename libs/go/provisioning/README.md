# Go Provisioning Boundary

Go provisioning services live here in the target structure.

This package owns provider-neutral Cloud provisioning rules: request
validation, deterministic idempotency keys, ordered provisioning steps, required
audit event names, SLA classification and rollback event naming.

Provider-specific adapters must live outside this package.
