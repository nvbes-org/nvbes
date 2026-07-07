# nvbes OSS Documentation

This directory is the public documentation source for `nvbes-oss`.

It is exported to `nvbes-oss/docs` and must not link to private blueprints, cloud runbooks or internal operational documents.

## Topics

- Installation with Docker Compose.
- Installation with Kubernetes and Helm.
- Self-hosted administration.
- Email delivery through `NVBES_EMAIL_PROVIDER=smtp`.
- Product analytics core without a bundled proprietary backend.
- Upgrade and backup procedures.
- Troubleshooting.

Cloud hosting, managed SLA, internal incident response and private roadmap documents stay outside the OSS export.

## Runtime Names

Public OSS documentation uses the target service names:

- `account-service`, `account-web`, `account-worker`;
- `cloud-service`, `cloud-web`, `cloud-worker`;
- `billing-service`, `billing-worker`;
- `developer-service`;
- `console-web`;
- `enterprise-service`;
- `enterprise-web`;
- `gateway-cloud`.

Do not document legacy runtime names in public OSS material. The OSS surface
must describe Account as the identity/session/account boundary, Cloud as the
workspace and file/product boundary, Billing as billing source of truth, and
Gateway Cloud as a stateless client composition layer.
