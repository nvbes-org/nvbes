# Snapshot Manifest

## Status

Template initialized. No source snapshot is approved for production migration.

- `total_source_snapshot_rows`: 1
- `source_snapshot_no_go_rows`: 1
- `source_snapshot_pending_rows`: 0
- `source_snapshot_passed_rows`: 0
- `source_snapshot_accepted_rows`: 0
- `source_snapshot_failed_rows`: 0
- `source_snapshot_blocking_rows`: 0
- `total_target_version_rows`: 1
- `target_version_no_go_rows`: 1
- `target_version_pending_rows`: 0
- `target_version_passed_rows`: 0
- `target_version_accepted_rows`: 0
- `target_version_failed_rows`: 0
- `target_version_blocking_rows`: 0
- `total_restore_rows`: 1
- `restore_go_decisions`: 0
- `restore_no_go_decisions`: 1

## Rules

- Every rehearsal and cutover run must reference an immutable source snapshot.
- A snapshot is invalid without source system, creation time, owner, checksum or
  storage proof, retention class and restore command.
- A `passed` or `accepted` snapshot row requires environment, source, creation
  time, owner, retention and checksum or storage proof.
- A `passed` or `accepted` snapshot row requires parseable creation time and
  concrete checksum or storage proof.
- A `passed` or `accepted` target version requires commit, image digests, SQL
  migrations, contracts and artifact check.
- A `passed` or `accepted` target version requires concrete commit, image and
  artifact-check references.
- A restore `go` decision requires snapshot, restore command, duration,
  environment and evidence.
- A restore `go` decision requires concrete evidence such as a report, result,
  log, artifact, command output, link or record.
- A restore `go` decision requires every source snapshot and target version row
  to be `passed` or `accepted`.
- The target version must include commit, image digests, SQL migration set,
  contract versions and artifact checks.
- Production cutover cannot start if the final snapshot is writable or lacks
  restore evidence.
- Snapshot retention and deletion must be approved by Data lead and Security
  lead.

## Source Snapshots

| Snapshot | Environment | Source | Created At | Owner | Checksum | Storage Proof | Retention | Status |
|---|---|---|---|---|---|---|---|---|
| final snapshot required | named production or rehearsal source environment with region and isolation record | legacy source systems, table/export scope and object-storage prefix scope for the migration run | creation timestamp artifact required before run with timezone and command output log | Data lead | checksum manifest artifact required for exports, tables and sampled objects | immutable storage proof required with bucket/path, object version IDs and retention-lock record | retention class, expiry timestamp and Data/Security approval record required | no-go |

## Target Versions

| Run | Commit | Images | SQL Migrations | Contracts | Artifact Check | Status |
|---|---|---|---|---|---|---|
| final target version required | signed target commit SHA, repository tag and CI run URL required | immutable OCI image digest manifest required for APIs, workers and frontend artifacts | SQL migration version report required with database targets, dry-run output and rollback compatibility result | OpenAPI, Protobuf and event schema contract report required with checksums and compatibility results | artifact check result required with source archive checksum, SBOM reference and release-freeze record | no-go |

## Restore Evidence

| Snapshot | Restore Command | Duration | Environment | Evidence | Decision |
|---|---|---|---|---|---|
| final snapshot required | restore command output log required with command, operator and target snapshot identifier | measured start/end duration required with rollback-window comparison | isolated restore environment required with region, network isolation and target version record | pending | no-go |

## Decision

Current decision: no-go.

No rehearsal, rollback or cutover can be approved until the relevant snapshot,
target version and restore evidence rows are completed and signed.

## Verification

```bash
pnpm check:migration-snapshots -- --strict
```
