# Backup Restore Manifest

## Status

Initialized. No backup or restore path is approved for production cutover.

- `total_decision_rows`: 11
- `pending_rows`: 11
- `passed_rows`: 0
- `accepted_rows`: 0
- `failed_rows`: 0
- `blocking_rows`: 0
- `go_decisions`: 0
- `no_go_decisions`: 11

This manifest captures the evidence required by Phase 5 before the final Big
Bang window: encrypted backups, restore commands, measured restore duration and
owner sign-off for every critical data plane.

## Rules

- Every critical store needs an encrypted backup, restore command and owner.
- Restore must be tested on the target staging environment before production.
- Backup age and restore duration must fit the announced cutover window.
- A `go` target requires backup proof, restore command, RPO, RTO and `passed`
  or `accepted` status.
- A `go` restore evidence row requires `passed` or `accepted` status plus
  concrete artifact content such as logs, reports, checksums, snapshots,
  command output, measured results, links or records.
- A `go` restore evidence row requires every backup target to be marked `go`.
- `pending`, missing evidence or `no-go` blocks production cutover.
- Rollback evidence may reference this manifest only after restore is measured.

## Backup Targets

| Target | Scope | Owner | Backup proof | Restore command | RPO | RTO | Status | Decision |
|---|---|---|---|---|---|---|---|---|
| PostgreSQL identity | users, tenants, auth, billing tables and migration metadata | Data lead | encrypted snapshot ID, WAL position, checksum manifest and immutable storage reference with retention lock | restore command transcript against isolated target with database version, target host and schema checksum output | measured WAL or snapshot age report within cutover window with timestamp and source LSN | measured restore duration report within rollback window with start/end timestamps and bottleneck notes | pending | no-go |
| PostgreSQL drive | metadata, shares, quotas, folder trees and object metadata references | Data lead | encrypted snapshot ID, WAL position, checksum manifest and immutable storage reference with retention lock | restore command transcript against isolated target with database version, target host and schema checksum output | measured WAL or snapshot age report within cutover window with timestamp and source LSN | measured restore duration report within rollback window with start/end timestamps and bottleneck notes | pending | no-go |
| object storage | files, exports, media, backup artifacts and sampled object prefixes | Infra lead | bucket inventory snapshot, object checksum sample, versioned object list and retention lock reference | restore/sync command transcript with sampled object verification, missing-object report and target prefix inventory | measured replication or inventory age report within cutover window with object-count timestamp | measured restore/sync duration report within rollback window with sampled checksum result | pending | no-go |
| Valkey | sessions, cache, locks, rate-limit counters and rebuildable key classes | Infra lead | encrypted RDB/AOF snapshot reference, checksum, key-count report and rebuild policy record | restore command transcript with key-count verification, TTL sample and rebuild command output | measured snapshot age or rebuild policy record with maximum tolerated cache/session loss | measured restore or rebuild duration report within rollback window with health result | pending | no-go |
| event log | outbox, topics, consumer offsets, DLQ and schema registry state | Infra lead | topic offset snapshot, schema registry export, DLQ inventory and retention reference | restore/replay command transcript with offset verification, consumer checkpoint result and DLQ replay report | measured offset lag and retention coverage report for every critical topic | measured replay duration report within rollback window with duplicate/idempotency result | pending | no-go |
| audit store | append-only audit evidence, export records and integrity chain | Security lead | immutable audit backup reference, checksum, retention proof and integrity-chain snapshot | restore command transcript with append-only integrity verification, export sample and chain validation output | measured backup age report within compliance window with retention timestamp | measured restore duration report within rollback window with integrity validation result | pending | no-go |

## Restore Evidence

| Evidence | Required content | Status | Decision |
|---|---|---|---|
| restore environment | isolated target artifact with region, network isolation proof, target version, secret scope, database/object-store endpoints and operator access record | pending | no-go |
| restore duration | measured start/finish timestamp report, command output log, bottleneck notes and rollback-window comparison for every critical target | pending | no-go |
| integrity checks | counts, checksums, schema checks, object sample proof, event offset comparison and audit integrity-chain validation report | pending | no-go |
| access controls | least-privilege restore operator list, approval record, temporary credential expiry, access review result and post-restore revocation proof | pending | no-go |
| retention proof | retention class, expiry date, lock policy, immutable reference, deletion approval path and Data/Security owner acknowledgement record | pending | no-go |

## Decision

Production cutover remains no-go until every backup target and restore evidence
row is signed, evidenced and marked `go`.

## Verification

```bash
pnpm check:migration-backup-restore -- --strict
```
