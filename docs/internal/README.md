# Internal Documentation

This directory is for internal operational material.

It can contain internal runbooks, incident procedures, internal architecture notes, business decisions and sensitive operational context.

## Runtime Taxonomy

Internal documentation uses `backoffice-service` and `backoffice-web` for
internal operations, support tooling, fraud/risk operations and internal admin
workflows.

Backoffice may compose internal views over Account, Cloud, Billing, Developer
and Enterprise through internal contracts or read models. It must not become the
source of truth for customer domains.

The old `backoffice-service` and `backoffice-web` names are allowed only in
migration evidence until the old-name deletion gate.
