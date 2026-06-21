# nvbes OSS Deployments

This directory contains deployment entrypoints that are safe to export to `nvbes-oss`.

## Targets

- `compose`: local and small self-hosted deployments.
- `helm`: Kubernetes package.
- `kustomize`: Kubernetes overlays for operators.
- `opentofu`: infrastructure scaffolding without Terraform lock-in.

Provider-specific managed cloud manifests belong in `deploy/cloud`, not here.
