# nvbes OSS

nvbes OSS is the self-hostable distribution of nvbes product capabilities.

The private `nvbes` monorepo is the source of truth. This public distribution is generated from an explicit allowlist and excludes private blueprints, cloud operations runbooks, internal docs, secrets and managed-cloud adapters.

## Current Scope

- Identity and Drive product services.
- Web applications and SDKs needed by the OSS product surface.
- OSS adapters and deployment targets.
- Public self-hosted documentation under `docs/`.

## Install Targets

- Docker Compose: `deploy/compose`.
- Kubernetes/Helm: `deploy/helm`.
- Kustomize and OpenTofu scaffolds for production operators.

The published OSS repository should be installed from released container images. Local development still uses the private monorepo workflows until the public release pipeline is complete.

## Boundaries

- OSS code must not depend on Cloud or Internal code.
- Cloud code may depend on OSS code.
- Internal code may depend on OSS and Cloud code.

Report security issues through the process in `SECURITY.md`.
