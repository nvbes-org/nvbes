# Security Policy

## Supported versions

Security fixes target the current `main` branch and the latest published
release, when one exists. Archived prototypes are out of support unless the
same issue also affects an active component.

## Reporting

Do not open public issues for suspected vulnerabilities. Send reports to
`security@nvbes.cloud`.

Send reports to the private security contact configured for the project maintainers. Include:

- affected component;
- reproduction steps;
- impact;
- logs or screenshots when safe;
- whether credentials, personal data or tenant isolation may be affected.

Encrypt sensitive attachments or reproduction material before sending them.
Do not access data that is not yours, disrupt production, perform denial of
service, or retain personal data beyond what is needed to demonstrate the issue.

The project does not currently operate a bug bounty and cannot promise payment.
Good-faith research that respects these boundaries will be handled through the
coordinated disclosure process referenced below.

## Scope

Security reports are accepted for:

- nvbes application services and workers;
- deployment and infrastructure templates;
- authentication and authorization flows;
- storage, billing, audit and tenant-isolation behavior;
- public SDKs and frontend clients.

The coordinated disclosure policy, safe harbor, response targets and bounty
status are documented in
`docs/compliance/vulnerability-disclosure-policy.md`.
