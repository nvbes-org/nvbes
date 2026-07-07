export const completedDomainEvidence = {
	Identity: {
		owner: "Identity lead",
		status: "passed",
		implementation_evidence: [
			"apps/account-service/src/identity.domains.auth.routes.register.rs",
			"apps/account-service/src/identity.http.middleware.jwt.session_refresh.rs",
			"apps/account-service/src/identity.domains.auth.sessions.mgmt.rs",
			"apps/account-service/src/identity.domains.auth.routes.login.mfa_flow.rs",
			"apps/account-service/openapi.json",
		],
		migration_evidence: [
			"docs/migration/identity-register.generated.json",
			"docs/migration/identity-login-session.generated.json",
			"docs/migration/identity-mfa-webauthn.generated.json",
		],
		decision: "go",
		proof:
			"pnpm check:migration-identity-register && pnpm check:migration-identity-login-session && pnpm check:migration-identity-mfa-webauthn",
	},
	Workspace: {
		owner: "Workspace lead",
		status: "passed",
		implementation_evidence: [
			"libs/rust/core/src/authz.policy.rs",
			"libs/rust/core/src/authz.policy.tests.rs",
			"apps/account-service/src/identity.domains.authz.service.rs",
			"apps/account-service/src/identity.domains.authz.db.rs",
		],
		migration_evidence: [
			"docs/migration/workspace-membership-roles.generated.json",
			"docs/migration/workspace-last-owner.generated.json",
		],
		decision: "go",
		proof:
			"pnpm check:migration-workspace-membership-roles && pnpm check:migration-workspace-last-owner",
	},
	Drive: {
		owner: "Drive lead",
		status: "passed",
		implementation_evidence: [
			"apps/cloud-service/src/drive.domains.uploads.core.rs",
			"apps/cloud-service/src/drive.domains.files.transfer.range.rs",
			"apps/cloud-service/src/drive.domains.share_links.logic.tests.rs",
			"apps/cloud-service/src/drive.domains.quotas.logic.rs",
			"apps/cloud-worker/src/drive.workers.maintenance.rs",
			"apps/cloud-service/openapi.json",
		],
		migration_evidence: [
			"docs/migration/drive-upload-download.generated.json",
			"docs/migration/drive-share-revoke.generated.json",
			"docs/migration/drive-quotas.generated.json",
		],
		decision: "go",
		proof:
			"pnpm check:migration-drive-upload-download && pnpm check:migration-drive-share-revoke && pnpm check:migration-drive-quotas",
	},
	Billing: {
		owner: "Billing lead",
		status: "passed",
		implementation_evidence: [
			"libs/rust/billing/src/views.rs",
			"libs/rust/billing/src/types.rs",
			"apps/billing-service/src/billing.domains.public_workspace.rs",
			"apps/billing-service/src/billing.domains.webhooks.rs",
			"apps/billing-worker/src/billing.worker.jobs.rs",
		],
		migration_evidence: [
			"docs/migration/billing-entitlements.generated.json",
			"docs/migration/billing-webhook-idempotency.generated.json",
			"docs/migration/billing-multi-psp-continuity.generated.json",
			"docs/migration/billing-multi-psp-e2e.generated.json",
		],
		decision: "go",
		proof:
			"pnpm check:migration-billing-entitlements && pnpm check:migration-billing-webhook-idempotency && pnpm check:migration-billing-multi-psp-continuity && pnpm check:migration-billing-multi-psp-e2e",
	},
	Audit: {
		owner: "Audit lead",
		status: "passed",
		implementation_evidence: [
			"libs/rust/audit/src/lib.rs",
			"apps/account-service/src/identity.domains.auth.sessions.mgmt.rs",
			"apps/cloud-service/src/drive.domains.share_links.observability.rs",
		],
		migration_evidence: ["docs/migration/audit-append-only.generated.json"],
		decision: "go",
		proof: "pnpm check:migration-audit-append-only",
	},
	Privacy: {
		owner: "Privacy lead",
		status: "passed",
		implementation_evidence: [
			"apps/account-worker/src/identity.worker.jobs.process_data_export.rs",
			"apps/account-service/src/identity.domains.auth.sessions.mgmt.rs",
			"libs/rust/products/account/src/account.email.jobs.rs",
		],
		migration_evidence: ["docs/migration/privacy-export-delete.generated.json"],
		decision: "go",
		proof: "pnpm check:migration-privacy-export-delete",
	},
	"Developer Platform": {
		owner: "Developer Platform lead",
		status: "passed",
		implementation_evidence: [
			"apps/account-service/src/identity.domains.developer.routes.oauth.rs",
			"apps/account-service/src/identity.domains.developer.routes.tokens.rs",
			"apps/account-service/src/identity.domains.developer.routes.webhooks.rs",
			"apps/console-web/src/developer.router.tsx",
			"libs/ts/identity-sdk-core/openapi.json",
		],
		migration_evidence: [
			"docs/migration/developer-oauth-tokens.generated.json",
			"docs/migration/developer-signed-webhooks.generated.json",
		],
		decision: "go",
		proof:
			"pnpm check:migration-developer-oauth-tokens && pnpm check:migration-developer-signed-webhooks",
	},
	Cloud: {
		owner: "Cloud lead",
		status: "passed",
		implementation_evidence: [
			"docs/migration/cloud-provisioning.generated.json",
			"libs/go/provisioning/provisioning.go",
			"libs/go/provisioning/provisioning_test.go",
			"libs/go/control-plane/foundation.go",
			"apps/cloud-service/README.md",
			"apps/cloud-web/README.md",
		],
		migration_evidence: [
			"docs/migration/cloud-provisioning.generated.json",
			"docs/migration/resource-map.generated.json",
			"docs/migration/secret-map.generated.json",
		],
		decision: "go",
		proof: "pnpm check:migration-cloud-provisioning && pnpm check:go && pnpm check:oss-boundaries",
	},
};
