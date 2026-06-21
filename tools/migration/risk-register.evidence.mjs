export const riskEvidence = {
	"modele-data-legacy-ambigu": {
		owner: "Data lead",
		status: "mitigated",
		evidence:
			"docs/migration/data-migration-pipeline.generated.json; docs/migration/data-map.generated.json; docs/migration/rejects.md; pnpm check:migration-data-migration-pipeline",
		cutover_impact:
			"allowed for repository gates: every source table has a target, owner, reconciliation checks and reject rule; production still needs an executed reconciliation report",
	},
	"objets-storage-manquants": {
		owner: "Drive lead",
		status: "mitigated",
		evidence:
			"docs/migration/data-migration-pipeline.generated.json; docs/migration/drive-upload-download.generated.json; docs/migration/drive-share-revoke.generated.json",
		cutover_impact:
			"allowed for repository gates: object/link invariants are encoded in the data map and Drive evidence; production still needs object-store reconciliation counts",
	},
	"divergence-billing": {
		owner: "Billing lead",
		status: "mitigated",
		evidence:
			"docs/migration/data-migration-pipeline.generated.json; docs/migration/billing-entitlements.generated.json; docs/migration/billing-webhook-idempotency.generated.json; docs/migration/release-freeze-manifest.md",
		cutover_impact:
			"allowed for repository gates: ledger-balance reconciliation is required and billing mutation freeze is documented; production still needs accepted ledger reconciliation",
	},
	"sessions-incompatibles": {
		owner: "Product lead",
		status: "mitigated",
		evidence:
			"apps/identity-web/src/identity.return-to.ts; apps/identity-web/src/identity.return-to.test.js; apps/developer-web/src/developer.session.ts; apps/developer-web/src/__tests__/developer.session.test.ts",
		cutover_impact:
			"allowed: incompatible sessions are handled by explicit logout/session clearing and a validated login return flow",
	},
	"fuite-cloud-internal-dans-oss": {
		owner: "Security lead",
		status: "mitigated",
		evidence:
			"tools/oss-export/checks.mjs; tools/oss-export/manifest.json; scripts/check-oss-provider-boundaries.mjs; pnpm check:oss-boundaries",
		cutover_impact:
			"allowed: OSS export allowlist and provider boundary checks block private docs and Cloud/Internal leakage",
	},
	"event-replay-non-idempotent": {
		owner: "Infra lead",
		status: "mitigated",
		evidence:
			"docs/migration/job-map.generated.json; docs/migration/billing-webhook-idempotency.generated.json; docs/migration/developer-signed-webhooks.generated.json; pnpm check:migration-job-map",
		cutover_impact:
			"allowed for repository gates: critical jobs and webhook replay paths carry idempotency-key evidence; production still needs DLQ and lag evidence in rehearsals",
	},
	"rollback-lent": {
		owner: "infra lead required",
		status: "pending",
		evidence:
			"docs/migration/rollback-report.md; docs/migration/rehearsal-ledger.md; docs/migration/live-evidence-instances.generated.json; pnpm check:migration-rollback-report -- --strict",
		cutover_impact:
			"no-go until an infra owner attaches accepted rollback live evidence with timed rollback proof from staging rehearsal",
	},
};
