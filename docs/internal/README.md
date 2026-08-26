# Internal Documentation

This directory is for internal operational material.

It can contain internal runbooks, incident procedures, internal architecture notes, business decisions and sensitive operational context.

## Runtime Taxonomy

Active V1 internal operations use the Platform Operations boundary. One
`platform_owner` handles support, security, abuse, billing, appeals and FinOps
manually through audited commands.

Platform Operations may compose internal views over Identity, Account, Billing,
Email and Trust/Risk through authenticated contracts. It must not become the
source of truth for those domains or access their databases directly.

The archived `backoffice-service` and `backoffice-web` are historical inventory,
not an active implementation base. See the
[V1 product direction](../product/nvbes-product-strategy.md).
