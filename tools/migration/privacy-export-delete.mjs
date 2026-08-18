#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { readPackageScripts, validateProofCommand } from "./execution-backlog.proof.mjs";

const args = process.argv.slice(2);
const write = args.includes("--write");
const outputPath = "docs/migration/privacy-export-delete.generated.json";
const markdownPath = "docs/migration/privacy-export-delete.md";

const sources = {
	openapi: "apps/account-service-next/openapi.json",
	openapiExport: "apps/account-service-next/src/account.openapi.rs",
	exportRoutes: "apps/account-service-next/src/account.privacy.routes.rs",
	exportStore: "apps/account-service-next/src/account.privacy.db.rs",
	accountClosure: "apps/account-service-next/src/account.closure.db.rs",
	accountExport: "libs/rust/products/account/src/account.privacy.data_export.rs",
	accountExportQuery: "libs/rust/products/account/src/account.privacy.data_export.query.rs",
	identityExport: "apps/identity-service/src/identity.domains.auth.account_export.rs",
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
		openApiMethodCheck("openapi-export-request", "/api/v1/privacy/exports", "post", "OpenAPI exposes data export request"),
		openApiMethodCheck("openapi-export-download", "/api/v1/privacy/exports/{exportId}/document", "get", "OpenAPI exposes prepared export download"),
		textCheck("openapi-export-handler", sources.openapiExport, "OpenAPI includes privacy export handlers", "crate::privacy_routes::download_export"),
		textCheck("export-request", sources.exportRoutes, "Export request persists an Account-owned export", "crate::privacy_db::request"),
		textCheck("export-download", sources.exportRoutes, "Export download is principal scoped", "crate::privacy_db::document(&state.db, auth.principal_id"),
		textCheck("export-expiry", sources.exportStore, "Expired exports are purged", "DELETE FROM account_privacy_exports"),
		textCheck("export-participants", sources.exportStore, "Multi-product participants are ordered", "account_export_participants"),
		textCheck("export-cache-key-test", sources.accountExport, "Account export cache key is subject scoped", "account_export_cache_key_is_subject_scoped"),
		textCheck("export-cache-ttl-test", sources.accountExport, "Prepared export TTL is bounded", "account_export_ttl_is_one_day"),
		textCheck("identity-export", sources.identityExport, "Identity contributes privacy activity", "privacy_activity"),
		textCheck("closure-invalidates-export", sources.accountClosure, "Account closure invalidates prepared exports", "UPDATE account_privacy_exports"),
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
		"cargo test -p nvbes-product-account account_export --locked",
		"cargo test -p nvbes-account-service privacy --locked",
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
			"cargo test -p nvbes-product-account account_export --locked",
			"cargo test -p nvbes-account-service privacy --locked",
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
