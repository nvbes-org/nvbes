#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { readPackageScripts } from "./execution-backlog.proof.mjs";
import { serializeReadinessMarkdown } from "./readiness-report.markdown.mjs";
import { proofForReadinessArea, proofForReadinessBlocker, validateReadinessBlockerProof } from "./readiness-report.proof.mjs";

const args = process.argv.slice(2);
const write = args.includes("--write");
const strict = args.includes("--strict");
const jsonPath = "docs/migration/readiness-report.generated.json";
const markdownPath = "docs/migration/readiness-report.md";
const errors = [];
const packageScripts = readPackageScripts(errors);

const sourceFiles = [
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
	["developer_oauth_tokens", "docs/migration/developer-oauth-tokens.generated.json", "summary", "failed"],
	["developer_signed_webhooks", "docs/migration/developer-signed-webhooks.generated.json", "summary", "failed"],
	["cloud_provisioning", "docs/migration/cloud-provisioning.generated.json", "summary", "failed"],
	["phases", "docs/migration/phase-ledger.generated.json", "summary", "pending"],
	["domains", "docs/migration/domain-ledger.generated.json", "summary", "pending"],
	["domain_dod", "docs/migration/domain-dod.generated.json", "summary", "pending"],
	["risks", "docs/migration/risk-register.generated.json", "summary", "pending"],
	["gates", "docs/migration/gate-evidence.generated.json", "summary", "pending"],
];

function readJson(path) {
	if (!existsSync(path)) {
		errors.push(`${path}: missing`);
		return undefined;
	}
	try {
		return JSON.parse(readFileSync(path, "utf8"));
	} catch (error) {
		errors.push(`${path}: invalid JSON: ${error.message}`);
		return undefined;
	}
}

function countNoGoGates(gateEvidence) {
	return (gateEvidence?.gates ?? []).filter((gate) => gate.decision !== "go").length;
}

function sourceRows() {
	const rows = [];
	for (const [id, path, summaryKey, pendingKey] of sourceFiles) {
		const data = readJson(path);
		const summary = data?.[summaryKey] ?? {};
		rows.push({
			id,
			source: path,
			total:
				summary.entries ??
				summary.risks ??
				summary.gates ??
				summary.phases ??
				summary.domains ??
				summary.runtimes ??
				summary.checks ??
				0,
			pending: summary[pendingKey] ?? 0,
			proof: proofForReadinessArea(id),
		});
	}
	const gateEvidence = readJson("docs/migration/gate-evidence.generated.json");
	rows.push({ id: "gate_decisions", source: "docs/migration/gate-evidence.generated.json", total: gateEvidence?.summary?.gates ?? 0, pending: countNoGoGates(gateEvidence), proof: proofForReadinessArea("gate_decisions") });
	const reconciliation = readJson("docs/migration/reconciliation.template.json");
	rows.push({ id: "reconciliation_template", source: "docs/migration/reconciliation.template.json", total: 1, pending: reconciliation?.decision === "go" ? 0 : 1, proof: proofForReadinessArea("reconciliation_template") });
	const liveEvidence = readJson("docs/migration/live-evidence-instances.generated.json");
	rows.push({ id: "live_evidence", source: "docs/migration/live-evidence-instances.generated.json", total: liveEvidence?.status?.requirements ?? 0, pending: liveEvidence?.status?.missing_requirements ?? 1, proof: proofForReadinessArea("live_evidence") });
	return rows;
}

function buildReport() {
	const sources = sourceRows();
	const liveEvidenceSource = sources.find((source) => source.id === "live_evidence");
	const liveEvidence = readJson("docs/migration/live-evidence-instances.generated.json");
	const blockers = sources
		.filter((source) => source.pending > 0)
		.map((source) => ({
			area: source.id,
			source: source.source,
			blocking_items: source.pending,
			proof: proofForReadinessBlocker(source.id),
		}));
	return {
		schema_version: 1,
		generation: {
			command: "tools/migration/readiness-report.mjs --write",
			strict_cutover_command: "tools/migration/readiness-report.mjs --strict",
		},
		status: {
			production_cutover: blockers.length === 0 ? "go" : "no-go",
			blocking_areas: blockers.length,
			blocking_items: blockers.reduce((total, blocker) => total + blocker.blocking_items, 0),
			live_evidence_missing_requirements: liveEvidenceSource?.pending ?? null,
			live_evidence_missing_items: liveEvidence?.status?.missing_evidence_items ?? null,
		},
		sources,
		blockers,
	};
}

function serializeJson(report) {
	return `${JSON.stringify(report, null, 2)}\n`;
}

function validate(report) {
	if (report.schema_version !== 1) errors.push(`${jsonPath}: schema_version must be 1`);
	if (report.generation?.command !== "tools/migration/readiness-report.mjs --write") {
		errors.push(`${jsonPath}: generation.command is invalid`);
	}
	if (report.generation?.strict_cutover_command !== "tools/migration/readiness-report.mjs --strict") {
		errors.push(`${jsonPath}: generation.strict_cutover_command is invalid`);
	}
	if (!["go", "no-go"].includes(report.status.production_cutover)) {
		errors.push(`${jsonPath}: unsupported production_cutover status`);
	}
	for (const field of ["blocking_areas", "blocking_items"]) {
		if (!Number.isInteger(report.status[field]) || report.status[field] < 0) errors.push(`${jsonPath}: ${field} must be a non-negative integer`);
	}
	for (const field of ["live_evidence_missing_requirements", "live_evidence_missing_items"]) {
		if (!Number.isInteger(report.status[field]) || report.status[field] < 0) errors.push(`${jsonPath}: ${field} must be a non-negative integer`);
	}
	if (!Array.isArray(report.sources) || report.sources.length !== sourceFiles.length + 3) {
		errors.push(`${jsonPath}: sources must include all readiness inputs`);
	}
	if (!Array.isArray(report.blockers)) errors.push(`${jsonPath}: blockers must be an array`);
	validateSourceRows(report);
	validateBlockingConsistency(report);
	if (strict && report.status.production_cutover !== "go") {
		errors.push(`${jsonPath}: production cutover is ${report.status.production_cutover}`);
	}
}

function validateSourceRows(report) {
	const sources = Array.isArray(report.sources) ? report.sources : [];
	const expected = new Map([
		...sourceFiles.map(([id, source]) => [id, source]),
		["gate_decisions", "docs/migration/gate-evidence.generated.json"],
		["reconciliation_template", "docs/migration/reconciliation.template.json"],
		["live_evidence", "docs/migration/live-evidence-instances.generated.json"],
	]);
	const seen = new Set();
	for (const source of sources) {
		if (!source.id) errors.push(`${jsonPath}: source id is required`);
		if (seen.has(source.id)) errors.push(`${jsonPath}: duplicate source row ${source.id}`);
		seen.add(source.id);
		if (!expected.has(source.id)) {
			errors.push(`${jsonPath}: unexpected source row ${source.id}`);
		} else if (source.source !== expected.get(source.id)) {
			errors.push(`${source.id}: source path must be ${expected.get(source.id)}`);
		}
		if (!Number.isInteger(source.total) || source.total < 0) {
			errors.push(`${source.id}: total must be a non-negative integer`);
		}
		if (source.total === 0) errors.push(`${source.id}: total must include at least one checked item`);
		if (!Number.isInteger(source.pending) || source.pending < 0) {
			errors.push(`${source.id}: pending must be a non-negative integer`);
		}
		if (Number.isInteger(source.total) && Number.isInteger(source.pending) && source.pending > source.total) {
			errors.push(`${source.id}: pending cannot exceed total`);
		}
		if (!source.proof) errors.push(`${source.id}: proof is required`);
		else errors.push(...validateReadinessBlockerProof({ area: source.id, proof: source.proof }, packageScripts));
	}
	for (const id of expected.keys()) {
		if (!seen.has(id)) errors.push(`${jsonPath}: missing source row ${id}`);
	}
}

function validateBlockingConsistency(report) {
	const sources = Array.isArray(report.sources) ? report.sources : [];
	const blockers = Array.isArray(report.blockers) ? report.blockers : [];
	const sourceBlockers = sources.filter((source) => source.pending > 0);
	const blockingItems = blockers.reduce((total, blocker) => total + blocker.blocking_items, 0);
	if (report.status.blocking_areas !== blockers.length) {
		errors.push(`${jsonPath}: blocking_areas must match blocker rows`);
	}
	if (report.status.blocking_items !== blockingItems) {
		errors.push(`${jsonPath}: blocking_items must equal blocker row total`);
	}
	if (report.status.production_cutover === "go" && blockers.length > 0) {
		errors.push(`${jsonPath}: go status cannot include blockers`);
	}
	if (report.status.production_cutover === "no-go" && blockers.length === 0) {
		errors.push(`${jsonPath}: no-go status requires at least one blocker`);
	}
	if (blockers.length !== sourceBlockers.length) {
		errors.push(`${jsonPath}: blocker rows must match sources with blocking items`);
	}
	const sourcesByArea = new Map(sources.map((source) => [source.id, source]));
	const liveEvidenceSource = sourcesByArea.get("live_evidence");
	if (report.status.live_evidence_missing_requirements !== liveEvidenceSource?.pending) {
		errors.push(`${jsonPath}: live_evidence_missing_requirements must match live_evidence source blocking count`);
	}
	for (const blocker of blockers) {
		const source = sourcesByArea.get(blocker.area);
		if (!source) {
			errors.push(`${jsonPath}: blocker ${blocker.area} has no matching source`);
			continue;
		}
		if (blocker.source !== source.source) {
			errors.push(`${jsonPath}: blocker ${blocker.area} source must match source row`);
		}
		if (blocker.blocking_items !== source.pending) {
			errors.push(`${jsonPath}: blocker ${blocker.area} count must match source blocking count`);
		}
		errors.push(...validateReadinessBlockerProof(blocker, packageScripts));
	}
}

const report = buildReport();
const json = serializeJson(report);
const markdown = serializeReadinessMarkdown(report);

if (write) {
	mkdirSync(dirname(jsonPath), { recursive: true });
	writeFileSync(jsonPath, json);
	writeFileSync(markdownPath, markdown);
	console.log(`Migration readiness report written to ${markdownPath} and ${jsonPath}`);
	process.exit(0);
}

validate(report);

for (const [path, expected] of [[jsonPath, json], [markdownPath, markdown]]) {
	if (!existsSync(path)) {
		errors.push(`${path}: missing; run tools/migration/readiness-report.mjs --write`);
	} else if (readFileSync(path, "utf8") !== expected) {
		errors.push(`${path}: stale; run tools/migration/readiness-report.mjs --write`);
	}
}

if (errors.length > 0) {
	console.error("Migration readiness report checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(
	`Migration readiness report: ok (${report.status.production_cutover}, ${report.status.blocking_items} blocking items)`,
);
