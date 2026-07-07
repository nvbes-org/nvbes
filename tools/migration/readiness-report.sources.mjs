import { markdownDecisionRows } from "./markdown-decision-rows.mjs";
import { proofForReadinessArea } from "./readiness-report.proof.mjs";

const generatedSourceFiles = [
	["data", "docs/migration/data-map.generated.json", "summary", "pending"],
	["secrets", "docs/migration/secret-map.generated.json", "summary", "pending"],
	["jobs", "docs/migration/job-map.generated.json", "summary", "pending"],
	["resources", "docs/migration/resource-map.generated.json", "summary", "pending"],
	["target_structure", "docs/migration/target-structure.generated.json", "summary", "pending"],
	["codegen", "docs/migration/codegen.generated.json", "summary", "pending"],
	["supply_chain", "docs/migration/supply-chain.generated.json", "summary", "pending"],
	["runtimes", "docs/migration/runtime-foundation.generated.json", "summary", "pending"],
	["platform_primitives", "docs/migration/platform-primitives.generated.json", "summary", "pending"],
	["identity_register", "docs/migration/identity-register.generated.json", "summary", "failed"],
	["identity_login_session", "docs/migration/identity-login-session.generated.json", "summary", "failed"],
	["identity_mfa_webauthn", "docs/migration/identity-mfa-webauthn.generated.json", "summary", "failed"],
	["workspace_membership_roles", "docs/migration/workspace-membership-roles.generated.json", "summary", "failed"],
	["workspace_last_owner", "docs/migration/workspace-last-owner.generated.json", "summary", "failed"],
	["drive_upload_download", "docs/migration/drive-upload-download.generated.json", "summary", "failed"],
	["drive_share_revoke", "docs/migration/drive-share-revoke.generated.json", "summary", "failed"],
	["drive_quotas", "docs/migration/drive-quotas.generated.json", "summary", "failed"],
	["audit_append_only", "docs/migration/audit-append-only.generated.json", "summary", "failed"],
	["privacy_export_delete", "docs/migration/privacy-export-delete.generated.json", "summary", "failed"],
	["billing_entitlements", "docs/migration/billing-entitlements.generated.json", "summary", "failed"],
	["billing_webhook_idempotency", "docs/migration/billing-webhook-idempotency.generated.json", "summary", "failed"],
	["billing_multi_psp_continuity", "docs/migration/billing-multi-psp-continuity.generated.json", "summary", "failed"],
	["developer_oauth_tokens", "docs/migration/developer-oauth-tokens.generated.json", "summary", "failed"],
	["developer_signed_webhooks", "docs/migration/developer-signed-webhooks.generated.json", "summary", "failed"],
	["cloud_provisioning", "docs/migration/cloud-provisioning.generated.json", "summary", "failed"],
	["phases", "docs/migration/phase-ledger.generated.json", "summary", "pending"],
	["domains", "docs/migration/domain-ledger.generated.json", "summary", "pending"],
	["domain_dod", "docs/migration/domain-dod.generated.json", "summary", "pending"],
	["risks", "docs/migration/risk-register.generated.json", "summary", "pending"],
	["gates", "docs/migration/gate-evidence.generated.json", "summary", "pending"],
];

const operationalSourceFiles = [
	["owner_signoffs", "docs/migration/owner-signoff-matrix.md"],
	["cutover_checklist", "docs/migration/cutover-checklist.md"],
	["rehearsals", "docs/migration/rehearsal-ledger.md"],
	["snapshots", "docs/migration/snapshot-manifest.md"],
	["release_freeze", "docs/migration/release-freeze-manifest.md"],
	["communication", "docs/migration/communication-plan.md"],
	["observability", "docs/migration/observability-readiness.md"],
	["rejects", "docs/migration/rejects.md"],
	["reconciliation_report", "docs/migration/reconciliation-report.md"],
	["cutover_journal", "docs/migration/cutover-journal.md"],
];

export function buildReadinessSourceRows(readJson, readText) {
	const rows = [];
	for (const [id, path, summaryKey, pendingKey] of generatedSourceFiles) {
		const data = readJson(path);
		const summary = data?.[summaryKey] ?? {};
		rows.push({
			id,
			source: path,
			total: summary.entries ?? summary.risks ?? summary.gates ?? summary.phases
				?? summary.domains ?? summary.runtimes ?? summary.checks ?? 0,
			pending: summary[pendingKey] ?? 0,
			proof: proofForReadinessArea(id),
		});
	}
	rows.push(...operationalRows(readText));
	return rows;
}

export function expectedReadinessSources() {
	return new Map([
		...generatedSourceFiles.map(([id, source]) => [id, source]),
		...operationalSourceFiles.map(([id, source]) => [id, source]),
		["reconciliation_template", "docs/migration/reconciliation.template.json"],
		["live_evidence", "docs/migration/live-evidence-instances.generated.json"],
	]);
}

function operationalRows(readText) {
	return operationalSourceFiles.map(([id, source]) => {
		const rows = markdownDecisionRows(readText(source));
		return {
			id,
			source,
			total: rows.length,
			pending: rows.filter((row) => row.blocking).length,
			proof: proofForReadinessArea(id),
		};
	});
}
