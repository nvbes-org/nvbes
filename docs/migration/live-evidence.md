# Live Evidence Schema

## Status

- evidence_types: 11
- required_fields: 13
- cutover_packet_requirements: 6
- checksum_max_items: 4
- decision: no-go until production evidence instances validate against the schema

## Evidence Types

| Type | Purpose |
|---|---|
| frontend_signoff | G4 E2E and accessibility evidence |
| web_check | G4 full web format, lint and typecheck evidence |
| smoke_test | G4/G7 strict smoke-test evidence |
| infra_restore | G5 staging rebuild, backup and restore evidence |
| infra_deploy | G5 infrastructure deployment evidence |
| rehearsal | P11/G6 migration rehearsal evidence |
| rollback | rollback duration and recovery proof |
| reject_review | P11/G6 reject log review evidence |
| cutover | P12/G7 production cutover journal evidence |
| reconciliation | production or rehearsal reconciliation evidence |
| decommission | P13/G8 legacy shutdown evidence |

## Required Fields

| Field | Required |
|---|---:|
| schema_version | yes |
| evidence_id | yes |
| evidence_type | yes |
| environment | yes |
| captured_at | yes |
| owner | yes |
| source_artifact | yes |
| command | yes |
| result | yes |
| decision | yes |
| immutable_reference | yes |
| packet_requirement | yes |
| checksums | yes |

## Packet Requirements

| Requirement | Evidence Instance Required |
|---|---:|
| g4-frontend-signoff | yes |
| g5-infra-signoff | yes |
| p11-rehearsals | yes |
| p12-cutover | yes |
| p13-decommission | yes |
| final-reconciliation | yes |

## Decision

The schema is ready. Cutover remains no-go until each required live evidence instance exists, is immutable, and validates against this contract.

## Verification

```bash
pnpm check:migration-live-evidence-schema
pnpm check:migration-live-evidence-rules
```
