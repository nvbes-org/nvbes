# Boundary Checks

Architecture checks that enforce the Big Bang restructure rules.

## Checks

- `check-product-boundaries.mjs`: product libraries under `libs/*/products`
  must not depend on apps, Cloud/Internal code or adapters. It also blocks
  legacy service runtime names from executable runtime/configuration surfaces.
- `check-product-boundaries.legacy-runtime-names.mjs`: scans `.github`, `apps`,
  `libs`, `contracts`, `deploy`, `docs`, `infrastructure`, `scripts`, `tools`
  and root runtime config files for deleted runtime names, production hostnames and
  display labels such as the historical Account, Cloud, Backoffice and gateway
  aliases. Only `docs/migration/account-cloud-big-bang.inventory.md` may keep
  the legacy runtime inventory.
- `check-product-boundaries.gateway-cloud.mjs`: keeps Gateway Cloud as a
  stateless composition layer by blocking migrations, persistence crates, service
  runtime crate dependencies, database connection types and SQL statements.
- `check-product-boundaries.mjs`: requires every resource server validator to
  accept only its own OAuth audience. In particular, Cloud cannot accept an
  Account access token, and Account APIs cannot accept the Identity browser
  session in place of an OAuth bearer token.
- `check-product-boundaries.mjs`: requires OAuth credentials and unverified
  identity lifecycle logic to live in `nvbes-product-identity`; Account cannot
  own or re-export these Identity capabilities.
- `scripts/check-oss-internal-imports.mjs`: scans OSS and Cloud source projects
  for direct imports, `require`, dynamic imports, Rust `#[path]`, and include
  macros that point at `scope:internal` project packages or source paths.

These checks are intentionally executable. Runtime surfaces, generated migration
evidence and docs must use the Account, Cloud, Billing, Developer, Enterprise,
Backoffice and Gateway Cloud names unless they are the explicit legacy inventory.
