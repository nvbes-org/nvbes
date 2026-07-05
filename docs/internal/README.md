# Internal Documentation

This directory is for private operational material.

It can contain internal runbooks, incident procedures, private architecture notes, business decisions and sensitive operational context.

It is never exported to `nvbes-oss`.

## Runtime Taxonomy

Internal documentation uses `backoffice-service` and `backoffice-web` for
internal operations, support tooling, fraud/risk operations and internal admin
workflows.

Backoffice may compose internal views over Account, Cloud, Billing, Developer
and Enterprise through internal contracts or read models. It must not become the
source of truth for customer domains, and `scope:internal` code must not be
imported by OSS or Cloud projects.

The old `internal-admin` and `internal-admin-web` names are allowed only in
migration evidence until the old-name deletion gate.
