export const gateEvidence = {
	G0: {
		owner: "Migration lead",
		status: "passed",
		evidence: [
			"docs/migration/inventory.generated.json",
			"docs/migration/data-map.generated.json",
			"docs/migration/secret-map.generated.json",
			"docs/migration/job-map.generated.json",
			"docs/migration/resource-map.generated.json",
			"docs/migration/owner-signoff-matrix.md",
			"docs/migration/parity-matrix.md",
		],
		decision: "go",
		notes:
			"Repository freeze gate is satisfied by generated inventory and decision maps with zero pending source data, secret, job, or resource decisions.",
	},
	G1: {
		owner: "Platform lead",
		status: "passed",
		evidence: [
			"docs/migration/target-structure.generated.json",
			"docs/migration/codegen.generated.json",
			"docs/migration/supply-chain.generated.json",
			"docs/migration/runtime-foundation.generated.json",
			"tools/boundary-checks/check-product-boundaries.mjs",
			"scripts/check-nx-boundaries.mjs",
			"scripts/check-rust-oss-boundaries.mjs",
			"tools/oss-export/checks.mjs",
		],
		decision: "go",
		notes:
			"Automated foundation gate is satisfied by active monorepo workspaces, codegen, supply-chain, runtime and boundary checks.",
	},
	G2: {
		owner: "Platform lead",
		status: "passed",
		evidence: [
			"docs/migration/platform-primitives.generated.json",
			"libs/rust/audit/src/lib.rs",
			"libs/rust/platform/src/platform.outbox.rs",
			"libs/rust/ports/src/ports.outbox.rs",
			"libs/rust/tenancy/src/lib.rs",
			"libs/rust/core/src/http.error.rs",
			"libs/rust/core/src/idempotency.rs",
			"libs/rust/observability/src/lib.rs",
			"contracts/events/manifest.json",
		],
		decision: "go",
		notes:
			"Primitive gate is satisfied by tested audit, outbox, tenancy, error, observability, idempotency and event-schema controls.",
	},
	G3: {
		owner: "Migration lead",
		status: "passed",
		evidence: [
			"docs/migration/parity-matrix.md",
			"docs/migration/domain-ledger.generated.json",
			"docs/migration/domain-dod.generated.json",
			"docs/migration/codegen.generated.json",
			"contracts/openapi/manifest.json",
		],
		decision: "go",
		notes:
			"Domain gate is satisfied for repository controls by strict parity, generated public contracts and evidence-backed domain ledgers.",
	},
	G4: {
		owner: "Product leads",
		evidence: [
			"docs/migration/cutover-evidence-packet.generated.json",
			"docs/migration/live-evidence-instances.generated.json",
			"docs/migration/frontend-experience.generated.json",
			"docs/migration/smoke-test-manifest.md",
		],
		notes:
			"Pending frontend sign-off; requires accepted g4-frontend-signoff live evidence before gate approval.",
	},
	G5: {
		owner: "Infra lead",
		evidence: [
			"docs/migration/cutover-evidence-packet.generated.json",
			"docs/migration/live-evidence-instances.generated.json",
			"docs/migration/infra-deploy.generated.json",
			"docs/migration/backup-restore-manifest.md",
			"docs/migration/rollback-report.md",
		],
		notes:
			"Pending infra sign-off; requires accepted g5-infra-signoff live restore and rollback evidence before gate approval.",
	},
	G6: {
		owner: "Data lead",
		evidence: [
			"docs/migration/cutover-evidence-packet.generated.json",
			"docs/migration/live-evidence-instances.generated.json",
			"docs/migration/rehearsal-ledger.md",
			"docs/migration/reconciliation.template.json",
			"docs/migration/rejects.md",
		],
		notes:
			"Pending migration rehearsals; requires accepted p11-rehearsals live evidence before gate approval.",
	},
	G7: {
		owner: "Migration lead",
		evidence: [
			"docs/migration/cutover-evidence-packet.generated.json",
			"docs/migration/live-evidence-instances.generated.json",
			"docs/migration/cutover-journal.md",
			"docs/migration/cutover-checklist.md",
			"docs/migration/observability-readiness.md",
			"docs/migration/smoke-test-manifest.md",
		],
		notes:
			"Pending production cutover; requires accepted p12-cutover and final reconciliation live evidence before gate approval.",
	},
	G8: {
		owner: "Infra lead",
		evidence: [
			"docs/migration/cutover-evidence-packet.generated.json",
			"docs/migration/live-evidence-instances.generated.json",
			"docs/migration/decommission-manifest.md",
			"docs/migration/post-migration-audit.md",
		],
		notes:
			"Pending decommission; requires accepted p13-decommission live evidence before gate approval.",
	},
};
