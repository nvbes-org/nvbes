import { validateProofCommand } from "./execution-backlog.proof.mjs";

const readinessProofs = {
	audit_append_only: "pnpm check:migration-audit-append-only",
	billing_entitlements: "pnpm check:migration-billing-entitlements",
	billing_webhook_idempotency: "pnpm check:migration-billing-webhook-idempotency",
	cloud_provisioning: "pnpm check:migration-cloud-provisioning",
	codegen: "pnpm check:migration-codegen",
	data: "pnpm check:migration-data-map",
	developer_oauth_tokens: "pnpm check:migration-developer-oauth-tokens",
	developer_signed_webhooks: "pnpm check:migration-developer-signed-webhooks",
	domain_dod: "pnpm check:migration-domain-dod",
	domains: "pnpm check:migration-domain-ledger",
	drive_quotas: "pnpm check:migration-drive-quotas",
	drive_share_revoke: "pnpm check:migration-drive-share-revoke",
	drive_upload_download: "pnpm check:migration-drive-upload-download",
	gate_decisions: "tools/migration/gate-evidence.mjs --strict",
	gates: "tools/migration/gate-evidence.mjs --strict",
	identity_login_session: "pnpm check:migration-identity-login-session",
	identity_mfa_webauthn: "pnpm check:migration-identity-mfa-webauthn",
	identity_register: "pnpm check:migration-identity-register",
	jobs: "pnpm check:migration-job-map",
	live_evidence: "tools/migration/live-evidence-instances.mjs --strict",
	phases: "tools/migration/phase-ledger.mjs --strict",
	platform_primitives: "pnpm check:migration-platform-primitives",
	privacy_export_delete: "pnpm check:migration-privacy-export-delete",
	reconciliation_template: "tools/migration/reconcile.mjs --env production --report docs/migration/reconciliation.<run>.json",
	resources: "pnpm check:migration-resource-map",
	risks: "tools/migration/risk-register.mjs --strict",
	runtimes: "pnpm check:migration-runtime-foundation",
	secrets: "pnpm check:migration-secret-map",
	supply_chain: "pnpm check:migration-supply-chain",
	target_structure: "pnpm check:migration-target-structure",
	workspace_last_owner: "pnpm check:migration-workspace-last-owner",
	workspace_membership_roles: "pnpm check:migration-workspace-membership-roles",
};

export function proofForReadinessArea(area) {
	return readinessProofs[area];
}

export function proofForReadinessBlocker(area) {
	return proofForReadinessArea(area);
}

export function validateReadinessBlockerProof(blocker, scripts) {
	if (!blocker?.proof) return [`${blocker?.area ?? "blocker"}: proof is required`];
	return validateProofCommand({ id: `readiness-${blocker.area}`, proof: blocker.proof }, scripts);
}
