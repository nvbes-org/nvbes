#!/usr/bin/env node
import { spawnSync } from "node:child_process";

const generators = [
	["inventory", "tools/migration/inventory.mjs", ["--write"]],
	["data map", "tools/migration/data-map.mjs", ["--write"]],
	["secret map", "tools/migration/secret-map.mjs", ["--write"]],
	["job map", "tools/migration/job-map.mjs", ["--write"]],
	["resource map", "tools/migration/resource-map.mjs", ["--write"]],
	["data migration pipeline", "tools/migration/data-migration-pipeline.mjs", ["--write"]],
	["target structure", "tools/migration/target-structure.mjs", ["--write"]],
	["codegen", "tools/migration/codegen.mjs", ["--write"]],
	["SBOM", "tools/security/sbom.mjs", ["--write"]],
	["supply chain", "tools/migration/supply-chain.mjs", ["--write"]],
	["runtime foundation", "tools/migration/runtime-foundation.mjs", ["--write"]],
	["platform primitives", "tools/migration/platform-primitives.mjs", ["--write"]],
	["identity register", "tools/migration/identity-register.mjs", ["--write"]],
	["identity login/session", "tools/migration/identity-login-session.mjs", ["--write"]],
	["identity MFA/WebAuthn", "tools/migration/identity-mfa-webauthn.mjs", ["--write"]],
	["workspace membership roles", "tools/migration/workspace-membership-roles.mjs", ["--write"]],
	["workspace last owner", "tools/migration/workspace-last-owner.mjs", ["--write"]],
	["drive upload/download", "tools/migration/drive-upload-download.mjs", ["--write"]],
	["drive share/revoke", "tools/migration/drive-share-revoke.mjs", ["--write"]],
	["drive quotas", "tools/migration/drive-quotas.mjs", ["--write"]],
	["audit append-only", "tools/migration/audit-append-only.mjs", ["--write"]],
	["privacy export/delete", "tools/migration/privacy-export-delete.mjs", ["--write"]],
	["billing entitlements", "tools/migration/billing-entitlements.mjs", ["--write"]],
	["billing webhook idempotency", "tools/migration/billing-webhook-idempotency.mjs", ["--write"]],
	["developer OAuth tokens", "tools/migration/developer-oauth-tokens.mjs", ["--write"]],
	["developer signed webhooks", "tools/migration/developer-signed-webhooks.mjs", ["--write"]],
	["cloud provisioning", "tools/migration/cloud-provisioning.mjs", ["--write"]],
	["frontend experience", "tools/migration/frontend-experience.mjs", ["--write"]],
	["infra deploy", "tools/migration/infra-deploy.mjs", ["--write"]],
	["phase ledger", "tools/migration/phase-ledger.mjs", ["--write"]],
	["domain ledger", "tools/migration/domain-ledger.mjs", ["--write"]],
	["domain DoD", "tools/migration/domain-dod.mjs", ["--write"]],
	["risk register", "tools/migration/risk-register.mjs", ["--write"]],
	["gate evidence", "tools/migration/gate-evidence.mjs", ["--write"]],
	["readiness report", "tools/migration/readiness-report.mjs", ["--write"]],
	["cutover evidence packet", "tools/migration/cutover-evidence-packet.mjs", ["--write"]],
	["live evidence schema", "tools/migration/live-evidence-schema.mjs", ["--write"]],
	["live evidence instances", "tools/migration/live-evidence-instances.mjs", ["--write"]],
	["completion audit", "tools/migration/completion-audit.mjs", ["--write"]],
	["execution backlog", "tools/migration/execution-backlog.mjs", ["--write"]],
	["inventory final pass", "tools/migration/inventory.mjs", ["--write"]],
];

for (const [label, script, args] of generators) {
	const result = spawnSync(process.execPath, [script, ...args], {
		encoding: "utf8",
		stdio: ["ignore", "pipe", "pipe"],
	});
	const output = `${result.stdout}${result.stderr}`.trim();
	if (result.status !== 0) {
		console.error(`Migration generator failed at ${label}:`);
		if (output) console.error(output);
		process.exit(1);
	}
	const line = output.split("\n").at(-1);
	console.log(`ok: ${label}${line ? ` (${line})` : ""}`);
}

console.log(`Migration generated artifacts refreshed (${generators.length} steps)`);
