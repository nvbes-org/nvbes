# Drive Share Revoke Evidence

## Status

- status: passed
- checks: 32
- passed: 32
- failed: 0

## Rules

- Every evidence row must be generated from the drive share/revoke source contract.
- `passed` requires the configured file to contain the expected pattern.
- Summary counters must match evidence rows.
- Targeted tests must name the share-link and reconciliation checks required by parity.
- Generation provenance must identify sources, write command and targeted tests.

## Evidence

| Check | Status | Path |
|---|---:|---|
| Share link expiration is normalized against workspace TTL policy | passed | `apps/cloud-service/src/drive.domains.share_links.logic.rs` |
| Past and over-policy expirations are covered by tests | passed | `apps/cloud-service/src/drive.domains.share_links.logic.tests.rs` |
| Max-download limits reject non-positive values | passed | `apps/cloud-service/src/drive.domains.share_links.logic.rs` |
| Max-download validation is covered by tests | passed | `apps/cloud-service/src/drive.domains.share_links.logic.tests.rs` |
| Share update guard rejects revoked or expired links | passed | `apps/cloud-service/src/drive.domains.share_links.logic.rs` |
| Revoked and expired mutation guard is covered by tests | passed | `apps/cloud-service/src/drive.domains.share_links.logic.tests.rs` |
| Public share access rejects revoked, expired, inactive, quarantined, or unclean links | passed | `apps/cloud-service/src/drive.domains.share_links.logic.rs` |
| Public access denial cases are covered by tests | passed | `apps/cloud-service/src/drive.domains.share_links.logic.tests.rs` |
| Public download URL creation enforces max-download limits | passed | `apps/cloud-service/src/drive.domains.share_links.logic.rs` |
| Download-limit boundary is covered by tests | passed | `apps/cloud-service/src/drive.domains.share_links.logic.tests.rs` |
| Share creation writes audit evidence | passed | `apps/cloud-service/src/drive.domains.share_links.manage.rs` |
| Share update writes audit evidence | passed | `apps/cloud-service/src/drive.domains.share_links.manage.rs` |
| Share revoke writes audit evidence | passed | `apps/cloud-service/src/drive.domains.share_links.manage.rs` |
| Share revoke is idempotency-safe through an already-revoked conflict | passed | `apps/cloud-service/src/drive.domains.share_links.manage.rs` |
| Share revoke persistence sets revoked_at | passed | `apps/cloud-service/src/drive.domains.share_links.db.rs` |
| Public share resolution uses token hashes | passed | `apps/cloud-service/src/drive.domains.share_links.db.queries.rs` |
| Denied public share access is audited | passed | `apps/cloud-service/src/drive.domains.share_links.public.rs` |
| Public download URL creation increments download counts | passed | `apps/cloud-service/src/drive.domains.share_links.public.rs` |
| Public download URL creation uses shared download-limit rule | passed | `apps/cloud-service/src/drive.domains.share_links.public.rs` |
| Managed create-share route is exposed | passed | `apps/cloud-service/src/drive.domains.share_links.routes.manage.rs` |
| Managed revoke-share route is exposed | passed | `apps/cloud-service/src/drive.domains.share_links.routes.manage.rs` |
| Public share resolve route is exposed | passed | `apps/cloud-service/src/drive.domains.share_links.routes.public.rs` |
| Public share download URL route is exposed | passed | `apps/cloud-service/src/drive.domains.share_links.routes.public.rs` |
| Public API exposes share-link creation | passed | `apps/cloud-service/src/drive.domains.public_api.routes.v1_handlers.share_links.rs` |
| Public API exposes share-link revocation | passed | `apps/cloud-service/src/drive.domains.public_api.routes.v1_handlers.share_links.rs` |
| Public API share mutations require share_links:write scope | passed | `apps/cloud-service/src/drive.domains.public_api.routes.v1_handlers.share_links.rs` |
| Public API revoke records API audit event | passed | `apps/cloud-service/src/drive.domains.public_api.routes.v1_handlers.share_links.rs` |
| Authorization maps share mutations to write permission | passed | `apps/cloud-service/src/drive.domains.authz.service.rs` |
| OpenAPI source includes public API share handlers | passed | `apps/cloud-service/src/drive.http.openapi.rs` |
| Generated OpenAPI includes public API create-share path | passed | `apps/cloud-service/openapi.json` |
| Generated OpenAPI includes public API revoke-share path | passed | `apps/cloud-service/openapi.json` |
| Share links are explicitly mapped for migration | passed | `docs/migration/data-map.md` |

## Decision

Drive share/revoke parity evidence is covered for repository cutover gates. Production cutover still requires accepted share-link reconciliation counts.

## Regeneration

```bash
pnpm check:migration-drive-share-revoke
node tools/migration/drive-share-revoke.mjs --write
```
