#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { readPackageScripts, validateProofCommand } from "./execution-backlog.proof.mjs";

const args = process.argv.slice(2);
const write = args.includes("--write");
const outputPath = "docs/migration/drive-quotas.generated.json";
const markdownPath = "docs/migration/drive-quotas.md";

const sources = {
	quotaLogic: "apps/drive-api/src/drive.domains.quotas.logic.rs",
	quotaService: "apps/drive-api/src/drive.domains.quotas.service.rs",
	quotaDb: "apps/drive-api/src/drive.domains.quotas.db.rs",
	quotaRoutes: "apps/drive-api/src/drive.domains.quotas.routes.rs",
	quotaTypes: "apps/drive-api/src/drive.domains.quotas.types.rs",
	quotaObservability: "apps/drive-api/src/drive.domains.quotas.observability.rs",
	uploadCore: "apps/drive-api/src/drive.domains.uploads.core.rs",
	uploadComplete: "apps/drive-api/src/drive.domains.uploads.lifecycle.complete.rs",
	uploadAppend: "apps/drive-api/src/drive.domains.uploads.lifecycle.tus.append.rs",
	downloadStream: "apps/drive-api/src/drive.domains.files.transfer.stream.rs",
	downloadUrl: "apps/drive-api/src/drive.domains.files.transfer.download_url.rs",
	sharePublic: "apps/drive-api/src/drive.domains.share_links.public.rs",
	publicApiQuota: "apps/drive-api/src/drive.domains.public_api.routes.v1_handlers.quotas_audit.rs",
	workerMaintenance: "apps/drive-worker/src/drive.workers.maintenance.rs",
	workerDispatch: "apps/drive-worker/src/drive.workers.executor.dispatch.rs",
	openapiSource: "apps/drive-api/src/drive.http.openapi.rs",
	openapiJson: "apps/drive-api/openapi.json",
	dataMap: "docs/migration/data-map.md",
	jobMap: "docs/migration/job-map.md",
};

const errors = [];
const packageScripts = readPackageScripts(errors);

function read(path) {
	if (!existsSync(path)) {
		errors.push(`${path}: missing`);
		return "";
	}
	return readFileSync(path, "utf8");
}

function textCheck(id, path, description, pattern) {
	const content = read(path);
	return {
		id,
		description,
		path,
		status: content.includes(pattern) ? "passed" : "failed",
		pattern,
	};
}

function buildChecks() {
	return [
		textCheck("quota-response", sources.quotaTypes, "Quota response exposes storage, bandwidth, block state, and alerts", "pub struct QuotaResponse"),
		textCheck("quota-route", sources.quotaRoutes, "Drive quota route is exposed", "path = \"/workspaces/{workspaceId}/quota\""),
		textCheck("quota-route-authz", sources.quotaRoutes, "Drive quota route requires ViewQuota authorization", "WorkspaceAction::ViewQuota"),
		textCheck("public-api-quota", sources.publicApiQuota, "Public API quota route is exposed", "path = \"/v1/workspaces/{workspaceId}/quota\""),
		textCheck("public-api-scope", sources.publicApiQuota, "Public API quota route requires quota:read scope", "\"quota:read\""),
		textCheck("quota-row", sources.quotaService, "Quota service ensures quota row before reading", "db::ensure_quota_row"),
		textCheck("quota-percent", sources.quotaService, "Quota service returns storage usage percent", "storage_usage_percent(used_storage_bytes, included_storage_bytes)"),
		textCheck("quota-blocked", sources.quotaService, "Quota service returns upload block state", "upload_blocked(used_storage_bytes, included_storage_bytes)"),
		textCheck("quota-alerts", sources.quotaService, "Quota service returns threshold alerts", "storage_alerts(used_storage_bytes, included_storage_bytes)"),
		textCheck("upload-guard", sources.quotaService, "Quota service blocks uploads that exceed storage limits", "quota_exceeded"),
		textCheck("critical-guard", sources.quotaService, "Quota service blocks uploads when already over critical threshold", "quota_critical_exceeded"),
		textCheck("upload-records-storage", sources.quotaService, "Completed uploads record storage usage events", "\"storage_bytes\""),
		textCheck("upload-records-file-count", sources.quotaService, "Completed uploads record file count usage events", "\"file_count\""),
		textCheck("release-records-negative-storage", sources.quotaService, "Deleted files release storage through negative usage events", "-input.released_storage_bytes"),
		textCheck("bandwidth-record", sources.quotaService, "Downloads and public access record bandwidth usage", "\"bandwidth_out_bytes\""),
		textCheck("quota-db-lock", sources.quotaDb, "Quota DB locks usage rows before mutation", "FOR UPDATE OF qu"),
		textCheck("quota-db-idempotency", sources.quotaDb, "Quota usage events are idempotent by workspace and key", "ON CONFLICT (workspace_id, idempotency_key)"),
		textCheck("quota-db-bandwidth-cache", sources.quotaDb, "Quota DB recalculates current-month bandwidth cache", "update_bandwidth_usage_cache"),
		textCheck("upload-core-guard", sources.uploadCore, "Upload creation checks quota before accepting upload", "ensure_upload_allowed_tx"),
		textCheck("upload-complete-guard", sources.uploadComplete, "Upload completion rechecks quota against actual size", "ensure_upload_allowed_tx(&mut tx, access.workspace_id, actual_size)"),
		textCheck("upload-complete-record", sources.uploadComplete, "Upload completion records quota usage", "record_file_uploaded_tx"),
		textCheck("tus-append-guard", sources.uploadAppend, "TUS append checks quota against actual size", "ensure_upload_allowed_tx(&mut tx, access.workspace_id, actual_size)"),
		textCheck("download-stream-bandwidth", sources.downloadStream, "File streaming records bandwidth usage", "record_bandwidth_out_tx"),
		textCheck("download-url-bandwidth", sources.downloadUrl, "Download URL flow records bandwidth usage", "record_bandwidth_out_tx"),
		textCheck("share-public-bandwidth", sources.sharePublic, "Public share access records bandwidth usage", "record_bandwidth_out_tx"),
		textCheck("threshold-audit", sources.quotaObservability, "Quota threshold crossings create audit events", "\"quota.storage_warning\""),
		textCheck("quota-warning-test", sources.quotaLogic, "Quota warning/critical alert logic is covered by tests", "storage_alerts_emit_warning_and_critical_levels"),
		textCheck("quota-blocked-test", sources.quotaLogic, "Quota upload block behavior is covered by tests", "upload_blocked_only_at_or_over_limit"),
		textCheck("quota-threshold-test", sources.quotaLogic, "Quota threshold crossing behavior is covered by tests", "crosses_threshold_only_when_entering_threshold"),
		textCheck("worker-job-constant", sources.workerMaintenance, "Quota recalculation job constant exists", "JOB_QUOTAS_RECALCULATE"),
		textCheck("worker-recalculate", sources.workerMaintenance, "Quota worker recalculates used storage from active files", "RECALCULATE_QUOTAS_SQL"),
		textCheck("worker-column-test", sources.workerMaintenance, "Quota worker tests the canonical used_storage_bytes column", "recalculate_quotas_updates_used_storage_bytes"),
		textCheck("worker-dispatch", sources.workerDispatch, "Worker dispatch executes quota recalculation job", "maintenance::JOB_QUOTAS_RECALCULATE"),
		textCheck("openapi-source", sources.openapiSource, "Drive OpenAPI source includes quota public API route", "crate::domains::public_api::v1_handlers::get_quota"),
		textCheck("openapi-json", sources.openapiJson, "Generated OpenAPI includes quota path", "\"/v1/workspaces/{workspaceId}/quota\""),
		textCheck("data-map-quota", sources.dataMap, "Quota usage has ledger-balance reconciliation decision", "`target-postgres:billing-usage.quota_usage`"),
		textCheck("job-map-quota", sources.jobMap, "Quota recalculation job is mapped", "quotas.recalculate"),
	];
}

function summarize(checks) {
	const failed = checks.filter((check) => check.status === "failed").length;
	return {
		checks: checks.length,
		passed: checks.length - failed,
		failed,
		status: failed === 0 ? "passed" : "failed",
	};
}

function sameItems(actual, expected) {
	return Array.isArray(actual) && actual.length === expected.length && actual.every((item, index) => item === expected[index]);
}

function validateReport(report) {
	const seen = new Set();
	for (const check of report.checks) {
		if (seen.has(check.id)) errors.push(`${outputPath}: duplicate check ${check.id}`);
		seen.add(check.id);
		if (!check.description) errors.push(`${check.id}: description is required`);
		if (!Object.values(sources).includes(check.path)) errors.push(`${check.id}: path is not in drive quotas source contract`);
		if (!["passed", "failed"].includes(check.status)) errors.push(`${check.id}: unsupported status ${check.status}`);
		if (!check.pattern) errors.push(`${check.id}: pattern is required`);
	}
	const expectedSummary = summarize(report.checks);
	for (const [field, value] of Object.entries(expectedSummary)) {
		if (report.summary[field] !== value) errors.push(`${outputPath}: summary.${field} must be ${value}`);
	}
	if (report.schema_version !== 1) errors.push(`${outputPath}: schema_version must be 1`);
	if (report.generation?.command !== "tools/migration/drive-quotas.mjs --write") {
		errors.push(`${outputPath}: generation.command is invalid`);
	}
	if (!sameItems(report.generation?.sources, Object.values(sources))) {
		errors.push(`${outputPath}: generation.sources must match drive quotas source contract`);
	}
	if (!sameItems(report.generation?.targeted_tests, [
		"cargo test -p nvbes-drive-api quota --locked",
		"cargo test -p nvbes-drive-worker recalculate_quotas_updates_used_storage_bytes --locked",
		"pnpm check:migration-reconciliation-report",
	])) {
		errors.push(`${outputPath}: generation.targeted_tests is invalid`);
	}
	for (const command of report.generation?.targeted_tests ?? []) {
		errors.push(...validateProofCommand({ id: "drive-quotas", proof: command }, packageScripts));
	}
}

function serializeJson(data) {
	return `${JSON.stringify(data, null, 2)}\n`;
}

function serializeMarkdown(data) {
	const lines = [
		"# Drive Quotas Evidence",
		"",
		"## Status",
		"",
		`- status: ${data.summary.status}`,
		`- checks: ${data.summary.checks}`,
		`- passed: ${data.summary.passed}`,
		`- failed: ${data.summary.failed}`,
		"",
		"## Rules",
		"",
		"- Every evidence row must be generated from the drive quotas source contract.",
		"- `passed` requires the configured file or OpenAPI document to contain the expected pattern.",
		"- Summary counters must match evidence rows.",
		"- Targeted tests must name quota and reconciliation checks required by parity.",
		"- Generation provenance must identify sources, write command and targeted tests.",
		"",
		"## Evidence",
		"",
		"| Check | Status | Path |",
		"|---|---:|---|",
	];
	for (const check of data.checks) {
		lines.push(`| ${check.description} | ${check.status} | \`${check.path}\` |`);
	}
	lines.push(
		"",
		"## Decision",
		"",
		data.summary.failed === 0
			? "Drive quota parity evidence is covered for repository cutover gates. Production cutover still requires accepted quota reconciliation counts."
			: "Drive quota parity evidence is blocked until failed checks pass.",
		"",
		"## Regeneration",
		"",
		"```bash",
		"pnpm check:migration-drive-quotas",
		"tools/migration/drive-quotas.mjs --write",
		"```",
		"",
	);
	return lines.join("\n");
}

const checks = buildChecks();
const summary = summarize(checks);
const report = {
	schema_version: 1,
	generation: {
		command: "tools/migration/drive-quotas.mjs --write",
		sources: Object.values(sources),
		targeted_tests: [
			"cargo test -p nvbes-drive-api quota --locked",
			"cargo test -p nvbes-drive-worker recalculate_quotas_updates_used_storage_bytes --locked",
			"pnpm check:migration-reconciliation-report",
		],
	},
	summary,
	checks,
};

validateReport(report);

const json = serializeJson(report);
const markdown = serializeMarkdown(report);

if (write) {
	mkdirSync(dirname(outputPath), { recursive: true });
	writeFileSync(outputPath, json);
	writeFileSync(markdownPath, markdown);
	console.log(`Drive quotas evidence written to ${markdownPath} and ${outputPath}`);
	process.exit(0);
}

for (const check of checks) {
	if (check.status === "failed") errors.push(`${check.path}: missing ${check.description}`);
}

for (const [path, expected] of [
	[outputPath, json],
	[markdownPath, markdown],
]) {
	if (!existsSync(path)) errors.push(`${path}: missing; run tools/migration/drive-quotas.mjs --write`);
	else if (readFileSync(path, "utf8") !== expected) errors.push(`${path}: stale; run tools/migration/drive-quotas.mjs --write`);
}

if (errors.length > 0) {
	console.error("Drive quotas evidence checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(`Drive quotas evidence: ok (${summary.passed}/${summary.checks} checks passed)`);
