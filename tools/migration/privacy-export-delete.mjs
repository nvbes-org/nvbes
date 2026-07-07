#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { readPackageScripts, validateProofCommand } from "./execution-backlog.proof.mjs";

const args = process.argv.slice(2);
const write = args.includes("--write");
const outputPath = "docs/migration/privacy-export-delete.generated.json";
const markdownPath = "docs/migration/privacy-export-delete.md";

const sources = {
	openapi: "apps/account-service/openapi.json",
	openapiExport: "apps/account-service/src/identity.http.openapi.rs",
	exportRoutes: "apps/account-service/src/identity.domains.auth.routes.session_mgmt.export.rs",
	deleteRoute: "apps/account-service/src/identity.domains.auth.routes.session_mgmt.profile.rs",
	dataExport: "apps/account-service/src/identity.domains.auth.data_export.rs",
	accountDeletion: "apps/account-service/src/identity.domains.auth.account_deletion.rs",
	emailJobs: "libs/rust/products/account/src/account.email.jobs.rs",
	workerJobs: "apps/account-worker/src/identity.worker.jobs.execute.rs",
	workerExport: "apps/account-worker/src/identity.worker.jobs.process_data_export.rs",
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

function parseJson(path, content) {
	try {
		return JSON.parse(content);
	} catch (error) {
		errors.push(`${path}: invalid JSON: ${error.message}`);
		return undefined;
	}
}

function openApiMethodCheck(id, pathKey, method, description) {
	const content = read(sources.openapi);
	if (!content) {
		return { id, description, path: sources.openapi, status: "failed", pattern: `${pathKey}.${method}` };
	}
	const api = parseJson(sources.openapi, content);
	return {
		id,
		description,
		path: sources.openapi,
		status: api.paths?.[pathKey]?.[method] ? "passed" : "failed",
		pattern: `paths['${pathKey}'].${method}`,
	};
}

function buildChecks() {
	return [
		openApiMethodCheck("openapi-export-request", "/auth/me/export", "post", "OpenAPI exposes data export request"),
		openApiMethodCheck("openapi-export-download", "/auth/me/export", "get", "OpenAPI exposes prepared export download"),
		openApiMethodCheck("openapi-account-delete", "/auth/me/delete", "post", "OpenAPI exposes account deletion request"),
		textCheck("openapi-export-handler", sources.openapiExport, "OpenAPI export includes data export handlers", "crate::domains::auth::routes::session_mgmt::me_export"),
		textCheck("openapi-delete-handler", sources.openapiExport, "OpenAPI export includes account delete handler", "crate::domains::auth::routes::session_mgmt::profile::me_delete"),
		textCheck("export-request-step-up", sources.dataExport, "Data export request requires recent step-up", "verification::require_recent_step_up(redis, auth, None)"),
		textCheck("export-request-rate-limit", sources.dataExport, "Data export request is rate limited", '"auth_me_export"'),
		textCheck("export-request-confirm-email", sources.dataExport, "Data export request enqueues confirmation email", "business_type: \"data_export\".to_string()"),
		textCheck("export-request-worker-job", sources.dataExport, "Data export request enqueues worker job", "enqueue_data_export_job_tx(redis, auth.user_id(), &auth.user_email)"),
		textCheck("export-job-idempotency", sources.emailJobs, "Data export worker job is idempotent per user", "idempotency_key: Some(format!(\"export:{}\", user_id))"),
		textCheck("export-download-step-up", sources.exportRoutes, "Export download requires recent step-up", "verification::require_recent_step_up(&state.redis, &auth, None)"),
		textCheck("export-download-no-store", sources.exportRoutes, "Export download disables caching", "header::CACHE_CONTROL, \"no-store\""),
		textCheck("export-cache-key-test", sources.dataExport, "API tests prove account export cache key scope", "account_export_cache_key_is_subject_scoped"),
		textCheck("export-cache-ttl-test", sources.dataExport, "API tests prove prepared export TTL", "account_export_ttl_is_one_day"),
		textCheck("worker-dispatch", sources.workerJobs, "Worker dispatch handles data export jobs", "JOB_DATA_EXPORT =>"),
		textCheck("worker-build-export", sources.workerExport, "Worker builds account export", "build_account_export(&state.db, payload.user_id)"),
		textCheck("worker-store-export", sources.workerExport, "Worker stores prepared export before notification", "store_account_export("),
		textCheck("worker-send-email", sources.workerExport, "Worker notifies the requester when export is ready", "Export de vos donnees - nvbes"),
		textCheck("worker-payload-parse-test", sources.workerExport, "Worker smoke test accepts valid export payload", "data_export_worker_payload_accepts_user_and_email"),
		textCheck("worker-payload-missing-test", sources.workerExport, "Worker smoke test rejects missing user id", "data_export_worker_payload_rejects_missing_user_id"),
		textCheck("worker-payload-invalid-test", sources.workerExport, "Worker smoke test rejects invalid user id", "data_export_worker_payload_rejects_invalid_user_id"),
		textCheck("delete-step-up", sources.accountDeletion, "Account deletion requires AAL2 step-up", "Some(nvbes_core::auth::Aal::Aal2)"),
		textCheck("delete-rate-limit", sources.accountDeletion, "Account deletion is rate limited", '"auth_me_delete"'),
		textCheck("delete-user-status", sources.accountDeletion, "Account deletion marks user deleted", "UPDATE users SET status = 'deleted'"),
		textCheck("delete-principal-status", sources.accountDeletion, "Account deletion marks principal deleted", "UPDATE principals SET status = 'deleted'"),
		textCheck("delete-session-revoke", sources.accountDeletion, "Account deletion revokes sessions transactionally", "sessions_mgmt::revoke_all_user_sessions_tx"),
		textCheck("delete-redis-session-clear", sources.accountDeletion, "Account deletion clears Redis sessions", "clear_user_sessions"),
		textCheck("delete-pubsub-user", sources.accountDeletion, "Account deletion publishes user suspension", "publish_user_suspended"),
		textCheck("delete-pubsub-workspaces", sources.accountDeletion, "Account deletion publishes owned workspace deletion", "publish_workspace_deleted"),
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
		if (!Object.values(sources).includes(check.path)) errors.push(`${check.id}: path is not in privacy export/delete source contract`);
		if (!["passed", "failed"].includes(check.status)) errors.push(`${check.id}: unsupported status ${check.status}`);
		if (!check.pattern) errors.push(`${check.id}: pattern is required`);
	}
	const expectedSummary = summarize(report.checks);
	for (const [field, value] of Object.entries(expectedSummary)) {
		if (report.summary[field] !== value) errors.push(`${outputPath}: summary.${field} must be ${value}`);
	}
	if (report.schema_version !== 1) errors.push(`${outputPath}: schema_version must be 1`);
	if (report.generation?.command !== "node tools/migration/privacy-export-delete.mjs --write") {
		errors.push(`${outputPath}: generation.command is invalid`);
	}
	if (!sameItems(report.generation?.sources, Object.values(sources))) {
		errors.push(`${outputPath}: generation.sources must match privacy export/delete source contract`);
	}
	if (!sameItems(report.generation?.targeted_tests, [
		"cargo test -p nvbes-account-worker data_export_worker_payload --locked",
		"cargo test -p nvbes-account-service account_export --locked",
	])) {
		errors.push(`${outputPath}: generation.targeted_tests is invalid`);
	}
	for (const command of report.generation?.targeted_tests ?? []) {
		errors.push(...validateProofCommand({ id: "privacy-export-delete", proof: command }, packageScripts));
	}
}

function serializeJson(data) {
	return `${JSON.stringify(data, null, 2)}\n`;
}

function serializeMarkdown(data) {
	const lines = [
		"# Privacy Export and Delete Evidence",
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
		"- Every evidence row must be generated from the privacy export/delete source contract.",
		"- `passed` requires the configured file or OpenAPI document to contain the expected pattern.",
		"- Summary counters must match evidence rows.",
		"- Targeted tests must name account export and worker payload checks required by parity.",
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
			? "Audit/Privacy export and delete request evidence is covered for repository cutover gates."
			: "Audit/Privacy export and delete request evidence is blocked until failed checks pass.",
		"",
		"## Regeneration",
		"",
		"```bash",
		"pnpm check:migration-privacy-export-delete",
		"node tools/migration/privacy-export-delete.mjs --write",
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
		command: "node tools/migration/privacy-export-delete.mjs --write",
		sources: Object.values(sources),
		targeted_tests: [
			"cargo test -p nvbes-account-worker data_export_worker_payload --locked",
			"cargo test -p nvbes-account-service account_export --locked",
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
	console.log(`Privacy export/delete evidence written to ${markdownPath} and ${outputPath}`);
	process.exit(0);
}

for (const check of checks) {
	if (check.status === "failed") errors.push(`${check.path}: missing ${check.description}`);
}

for (const [path, expected] of [
	[outputPath, json],
	[markdownPath, markdown],
]) {
	if (!existsSync(path)) errors.push(`${path}: missing; run node tools/migration/privacy-export-delete.mjs --write`);
	else if (readFileSync(path, "utf8") !== expected) errors.push(`${path}: stale; run node tools/migration/privacy-export-delete.mjs --write`);
}

if (errors.length > 0) {
	console.error("Privacy export/delete evidence checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(`Privacy export/delete evidence: ok (${summary.passed}/${summary.checks} checks passed)`);
