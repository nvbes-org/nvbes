export function evidenceForCriterion(entry, domain) {
	if (domain.status !== "passed" && domain.status !== "accepted") return entry;
	if (entry.criterion_id.includes("migrations-data")) {
		return dataMigrationEvidence(entry, domain);
	}

	const baseEvidence = [
		"docs/migration/domain-ledger.generated.json",
		...domain.implementation_evidence,
		...domain.migration_evidence,
	];
	const common = {
		...entry,
		owner: domain.owner,
		status: "passed",
		decision: "go",
	};

	if (entry.criterion_id.includes("modele-domaine")) {
		return {
			...common,
			evidence: [
				...baseEvidence,
				"tools/boundary-checks/check-product-boundaries.mjs",
				"scripts/check-oss-provider-boundaries.mjs",
			],
			proof: "pnpm check:product-boundaries && pnpm check:oss-boundaries && pnpm check:migration-domain-ledger",
		};
	}

	if (entry.criterion_id.includes("commandes-critiques")) {
		return {
			...common,
			evidence: [
				...baseEvidence,
				"docs/migration/platform-primitives.generated.json",
				"libs/rust/audit/src/lib.rs",
				"libs/rust/core/src/idempotency.rs",
			],
			proof: `${domain.proof} && pnpm check:migration-platform-primitives`,
		};
	}

	if (entry.criterion_id.includes("events-critiques")) {
		return {
			...common,
			evidence: [
				...baseEvidence,
				"docs/migration/codegen.generated.json",
				"docs/migration/platform-primitives.generated.json",
				"contracts/events/manifest.json",
				"libs/rust/platform/src/platform.outbox.rs",
			],
			proof: "pnpm check:contracts && pnpm check:migration-codegen && pnpm check:migration-platform-primitives",
		};
	}

	if (entry.criterion_id.includes("tables-portent")) {
		return {
			...common,
			evidence: [...baseEvidence, "docs/migration/data-map.generated.json"],
			proof: "pnpm check:migration-data-map && pnpm check:migration-domain-ledger",
		};
	}

	if (entry.criterion_id.includes("endpoints-publics")) {
		return {
			...common,
			evidence: [
				...baseEvidence,
				"docs/migration/codegen.generated.json",
				"contracts/openapi/manifest.json",
				"libs/ts/identity-sdk-core/src/types.gen.ts",
				"libs/rust/identity-sdk-backend/src/lib.rs",
				"libs/go/identity-sdk/sdk.go",
			],
			proof: "pnpm check:contracts && pnpm check:codegen && pnpm check:migration-codegen",
		};
	}

	if (entry.criterion_id.includes("invariants-metier")) {
		return {
			...common,
			evidence: baseEvidence,
			proof: domain.proof,
		};
	}

	if (entry.criterion_id.includes("erreurs-utilisent")) {
		return {
			...common,
			evidence: [
				...baseEvidence,
				"docs/migration/platform-primitives.generated.json",
				"libs/rust/core/src/http.error.rs",
			],
			proof: "pnpm check:migration-platform-primitives && pnpm check:api",
		};
	}

	return entry;
}

function dataMigrationEvidence(entry, domain) {
	if (domain.domain !== "Cloud") {
		return {
			...entry,
			owner: domain.owner,
			status: "passed",
			evidence: [
				"docs/migration/data-migration-pipeline.generated.json",
				"docs/migration/data-map.generated.json",
				"docs/migration/reconciliation-report.schema.json",
				"docs/migration/rejects.md",
			],
			decision: "go",
			proof:
				"pnpm check:migration-data-migration-pipeline && pnpm check:migration-data-map && pnpm check:migration-rejects && pnpm check:migration-reconciliation-report",
		};
	}
	return {
		...entry,
		owner: domain.owner,
		status: "accepted",
		evidence: [
			"docs/migration/cloud-provisioning.generated.json",
			"docs/migration/resource-map.generated.json",
			"docs/migration/secret-map.generated.json",
		],
		decision: "go",
		proof:
			"pnpm check:migration-cloud-provisioning && pnpm check:migration-resource-map && pnpm check:migration-secret-map",
	};
}
