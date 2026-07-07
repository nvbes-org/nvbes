export const completedPhaseEvidence = {
	P01: {
		owner: "Migration lead",
		status: "passed",
		evidence: [
			"docs/migration/inventory.generated.json",
			"docs/migration/data-map.generated.json",
			"docs/migration/secret-map.generated.json",
			"docs/migration/job-map.generated.json",
			"docs/migration/resource-map.generated.json",
		],
		decision: "go",
		proof:
			"pnpm check:migration-inventory && pnpm check:migration-data-map && pnpm check:migration-secret-map && pnpm check:migration-job-map && pnpm check:migration-resource-map",
	},
	P02: {
		owner: "Platform lead",
		status: "passed",
		evidence: [
			"docs/migration/target-structure.generated.json",
			"docs/migration/codegen.generated.json",
			"docs/migration/supply-chain.generated.json",
			"docs/migration/runtime-foundation.generated.json",
			"tools/boundary-checks/check-product-boundaries.mjs",
			"scripts/check-nx-boundaries.mjs",
		],
		decision: "go",
		proof:
			"pnpm check:migration-target-structure && pnpm check:migration-codegen && pnpm check:migration-supply-chain && pnpm check:migration-runtime-foundation && pnpm check:product-boundaries && pnpm check:oss-boundaries",
	},
	P03: {
		owner: "Platform lead",
		status: "passed",
		evidence: [
			"docs/migration/platform-primitives.generated.json",
			"libs/rust/platform/src/platform.outbox.rs",
			"libs/rust/ports/src/lib.rs",
			"libs/rust/audit/src/lib.rs",
			"libs/rust/tenancy/src/lib.rs",
			"libs/rust/core/src/http.error.rs",
			"libs/rust/observability/src/lib.rs",
		],
		decision: "go",
		proof:
			"pnpm check:migration-platform-primitives && cargo test -p nvbes-platform --locked && cargo test -p nvbes-ports --locked && cargo check --workspace --locked",
	},
	P04: {
		owner: "Identity lead",
		status: "passed",
		evidence: [
			"docs/migration/identity-register.generated.json",
			"docs/migration/identity-login-session.generated.json",
			"docs/migration/identity-mfa-webauthn.generated.json",
			"apps/account-service/openapi.json",
		],
		decision: "go",
		proof:
			"pnpm check:migration-identity-register && pnpm check:migration-identity-login-session && pnpm check:migration-identity-mfa-webauthn",
	},
	P05: {
		owner: "Workspace lead",
		status: "passed",
		evidence: [
			"docs/migration/workspace-membership-roles.generated.json",
			"docs/migration/workspace-last-owner.generated.json",
			"libs/rust/core/src/authz.policy.tests.rs",
		],
		decision: "go",
		proof:
			"pnpm check:migration-workspace-membership-roles && pnpm check:migration-workspace-last-owner",
	},
	P06: {
		owner: "Drive lead",
		status: "passed",
		evidence: [
			"docs/migration/drive-upload-download.generated.json",
			"docs/migration/drive-share-revoke.generated.json",
			"docs/migration/drive-quotas.generated.json",
			"apps/cloud-service/openapi.json",
		],
		decision: "go",
		proof:
			"pnpm check:migration-drive-upload-download && pnpm check:migration-drive-share-revoke && pnpm check:migration-drive-quotas",
	},
	P07: {
		owner: "Billing lead",
		status: "passed",
		evidence: [
			"docs/migration/billing-entitlements.generated.json",
			"docs/migration/billing-webhook-idempotency.generated.json",
			"docs/migration/billing-multi-psp-continuity.generated.json",
			"libs/rust/billing/src/views.rs",
		],
		decision: "go",
		proof:
			"pnpm check:migration-billing-entitlements && pnpm check:migration-billing-webhook-idempotency && pnpm check:migration-billing-multi-psp-continuity",
	},
	P08: {
		owner: "Developer Platform lead",
		status: "passed",
		evidence: [
			"docs/migration/developer-oauth-tokens.generated.json",
			"docs/migration/developer-signed-webhooks.generated.json",
			"apps/console-web/src/developer.router.tsx",
			"libs/ts/identity-sdk-core/openapi.json",
		],
		decision: "go",
		proof:
			"pnpm check:migration-developer-oauth-tokens && pnpm check:migration-developer-signed-webhooks",
	},
	P09: {
		owner: "Product frontend lead",
		status: "passed",
		evidence: [
			"docs/migration/frontend-experience.generated.json",
			"apps/account-web/e2e/critical.spec.ts",
			"apps/cloud-web/src/drive.router.tsx",
			"apps/console-web/src/developer.router.tsx",
			"apps/enterprise-web/src/enterprise.router.tsx",
			"apps/cloud-web/README.md",
			"apps/backoffice-service/README.md",
		],
		decision: "go",
		proof:
			"pnpm check:migration-frontend-experience && pnpm check:web && pnpm --dir apps/enterprise-web test",
	},
	P10: {
		owner: "Infra lead",
		status: "passed",
		evidence: [
			"docs/migration/infra-deploy.generated.json",
			"deploy/oss/helm/nvbes/Chart.yaml",
			"deploy/oss/kustomize/kustomization.yaml",
			"infrastructure/environments/staging/main.tf",
			"infrastructure/environments/production/alloy.config.alloy",
			"scripts/release-gate.sh",
			"scripts/migrate-staging.sh",
		],
		decision: "go",
		proof:
			"pnpm check:migration-infra-deploy && tofu -chdir=infrastructure/environments/development validate && tofu -chdir=infrastructure/environments/staging validate && pnpm check:supply-chain && pnpm check:migration-backup-restore && pnpm check:migration-observability",
	},
	P11: {
		owner: "Data lead",
		status: "pending",
		evidence: [
			"docs/migration/cutover-evidence-packet.generated.json",
			"docs/migration/live-evidence-instances.generated.json",
			"docs/migration/rehearsal-ledger.md",
			"docs/migration/reconciliation.template.json",
			"docs/migration/rejects.md",
		],
		decision: "no-go",
		proof:
			"pnpm check:migration-rehearsals -- --strict && node tools/migration/reconcile.mjs --env staging --report docs/migration/reconciliation.<run>.json && pnpm check:migration-rejects -- --strict",
	},
	P12: {
		owner: "Migration lead",
		status: "pending",
		evidence: [
			"docs/migration/cutover-evidence-packet.generated.json",
			"docs/migration/live-evidence-instances.generated.json",
			"docs/migration/cutover-journal.md",
			"docs/migration/cutover-checklist.md",
			"docs/migration/observability-readiness.md",
			"docs/migration/smoke-test-manifest.md",
		],
		decision: "no-go",
		proof:
			"pnpm check:migration-precutover -- --env production --reconciliation-report docs/migration/reconciliation.<run>.json",
	},
	P13: {
		owner: "Infra lead",
		status: "pending",
		evidence: [
			"docs/migration/cutover-evidence-packet.generated.json",
			"docs/migration/live-evidence-instances.generated.json",
			"docs/migration/decommission-manifest.md",
			"docs/migration/post-migration-audit.md",
		],
		decision: "no-go",
		proof: "pnpm check:migration-postcutover",
	},
};
