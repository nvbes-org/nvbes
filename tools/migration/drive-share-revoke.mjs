#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { readPackageScripts, validateProofCommand } from "./execution-backlog.proof.mjs";

const args = process.argv.slice(2);
const write = args.includes("--write");
const outputPath = "docs/migration/drive-share-revoke.generated.json";
const markdownPath = "docs/migration/drive-share-revoke.md";

const sources = {
	logic: "apps/cloud-service/src/drive.domains.share_links.logic.rs",
	logicTests: "apps/cloud-service/src/drive.domains.share_links.logic.tests.rs",
	manage: "apps/cloud-service/src/drive.domains.share_links.manage.rs",
	publicShare: "apps/cloud-service/src/drive.domains.share_links.public.rs",
	db: "apps/cloud-service/src/drive.domains.share_links.db.rs",
	queries: "apps/cloud-service/src/drive.domains.share_links.db.queries.rs",
	manageRoutes: "apps/cloud-service/src/drive.domains.share_links.routes.manage.rs",
	publicRoutes: "apps/cloud-service/src/drive.domains.share_links.routes.public.rs",
	publicApi: "apps/cloud-service/src/drive.domains.public_api.routes.v1_handlers.share_links.rs",
	authz: "apps/cloud-service/src/drive.domains.authz.service.rs",
	openapiSource: "apps/cloud-service/src/drive.http.openapi.rs",
	openapiJson: "apps/cloud-service/openapi.json",
	dataMap: "docs/migration/data-map.md",
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
		textCheck("normalize-expires", sources.logic, "Share link expiration is normalized against workspace TTL policy", "pub fn normalize_expires_at"),
		textCheck("normalize-expires-test", sources.logicTests, "Past and over-policy expirations are covered by tests", "normalize_expires_at_rejects_past_and_ttl_overflow"),
		textCheck("max-downloads", sources.logic, "Max-download limits reject non-positive values", "pub fn normalize_max_downloads"),
		textCheck("max-downloads-test", sources.logicTests, "Max-download validation is covered by tests", "normalize_max_downloads_rejects_zero_or_negative_limits"),
		textCheck("mutation-guard", sources.logic, "Share update guard rejects revoked or expired links", "ensure_link_not_revoked_or_expired"),
		textCheck("mutation-guard-test", sources.logicTests, "Revoked and expired mutation guard is covered by tests", "ensure_link_not_revoked_or_expired_blocks_mutation"),
		textCheck("public-access-guard", sources.logic, "Public share access rejects revoked, expired, inactive, quarantined, or unclean links", "pub fn enforce_public_share_access"),
		textCheck("public-access-test", sources.logicTests, "Public access denial cases are covered by tests", "enforce_public_share_access_rejects_revoked_expired_and_unclean_links"),
		textCheck("download-limit-rule", sources.logic, "Public download URL creation enforces max-download limits", "ensure_public_share_download_allowed"),
		textCheck("download-limit-test", sources.logicTests, "Download-limit boundary is covered by tests", "ensure_public_share_download_allowed_blocks_at_limit_only"),
		textCheck("create-audit", sources.manage, "Share creation writes audit evidence", "\"share_link.created\""),
		textCheck("update-audit", sources.manage, "Share update writes audit evidence", "\"share_link.updated\""),
		textCheck("revoke-audit", sources.manage, "Share revoke writes audit evidence", "\"share_link.revoked\""),
		textCheck("revoke-conflict", sources.manage, "Share revoke is idempotency-safe through an already-revoked conflict", "Share link is already revoked."),
		textCheck("db-revoke", sources.db, "Share revoke persistence sets revoked_at", "SET revoked_at = $3"),
		textCheck("token-hash-query", sources.queries, "Public share resolution uses token hashes", "WHERE sl.token_hash = $1"),
		textCheck("public-denied-audit", sources.publicShare, "Denied public share access is audited", "\"share_link.access_denied\""),
		textCheck("public-download-count", sources.publicShare, "Public download URL creation increments download counts", "increment_download_count_tx"),
		textCheck("public-download-limit", sources.publicShare, "Public download URL creation uses shared download-limit rule", "ensure_public_share_download_allowed(&resolved)"),
		textCheck("manage-create-route", sources.manageRoutes, "Managed create-share route is exposed", "path = \"/workspaces/{workspaceId}/objects/{objectId}/share-links\""),
		textCheck("manage-revoke-route", sources.manageRoutes, "Managed revoke-share route is exposed", "path = \"/workspaces/{workspaceId}/share-links/{shareLinkId}\""),
		textCheck("public-resolve-route", sources.publicRoutes, "Public share resolve route is exposed", "path = \"/public/shares/{token}\""),
		textCheck("public-download-route", sources.publicRoutes, "Public share download URL route is exposed", "path = \"/public/shares/{token}/download-url\""),
		textCheck("public-api-create", sources.publicApi, "Public API exposes share-link creation", "path = \"/v1/workspaces/{workspaceId}/objects/{objectId}/share-links\""),
		textCheck("public-api-revoke", sources.publicApi, "Public API exposes share-link revocation", "path = \"/v1/workspaces/{workspaceId}/share-links/{shareLinkId}\""),
		textCheck("public-api-write-scope", sources.publicApi, "Public API share mutations require share_links:write scope", "\"share_links:write\""),
		textCheck("public-api-revoke-event", sources.publicApi, "Public API revoke records API audit event", "\"api.share_link.revoked\""),
		textCheck("authz-write", sources.authz, "Authorization maps share mutations to write permission", "drive.share_links.write"),
		textCheck("openapi-source", sources.openapiSource, "OpenAPI source includes public API share handlers", "crate::domains::public_api::v1_handlers::create_share_link"),
		textCheck("openapi-create-json", sources.openapiJson, "Generated OpenAPI includes public API create-share path", "\"/v1/workspaces/{workspaceId}/objects/{objectId}/share-links\""),
		textCheck("openapi-revoke-json", sources.openapiJson, "Generated OpenAPI includes public API revoke-share path", "\"/v1/workspaces/{workspaceId}/share-links/{shareLinkId}\""),
		textCheck("data-map-share-links", sources.dataMap, "Share links are explicitly mapped for migration", "`target-postgres:drive.share_links`"),
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
		if (!Object.values(sources).includes(check.path)) errors.push(`${check.id}: path is not in drive share/revoke source contract`);
		if (!["passed", "failed"].includes(check.status)) errors.push(`${check.id}: unsupported status ${check.status}`);
		if (!check.pattern) errors.push(`${check.id}: pattern is required`);
	}
	const expectedSummary = summarize(report.checks);
	for (const [field, value] of Object.entries(expectedSummary)) {
		if (report.summary[field] !== value) errors.push(`${outputPath}: summary.${field} must be ${value}`);
	}
	if (report.schema_version !== 1) errors.push(`${outputPath}: schema_version must be 1`);
	if (report.generation?.command !== "tools/migration/drive-share-revoke.mjs --write") {
		errors.push(`${outputPath}: generation.command is invalid`);
	}
	if (!sameItems(report.generation?.sources, Object.values(sources))) {
		errors.push(`${outputPath}: generation.sources must match drive share/revoke source contract`);
	}
	if (!sameItems(report.generation?.targeted_tests, [
		"cargo test -p nvbes-cloud-service share_link --locked",
		"pnpm check:migration-reconciliation-report",
	])) {
		errors.push(`${outputPath}: generation.targeted_tests is invalid`);
	}
	for (const command of report.generation?.targeted_tests ?? []) {
		errors.push(...validateProofCommand({ id: "drive-share-revoke", proof: command }, packageScripts));
	}
}

function serializeJson(data) {
	return `${JSON.stringify(data, null, 2)}\n`;
}

function serializeMarkdown(data) {
	const lines = [
		"# Drive Share Revoke Evidence",
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
		"- Every evidence row must be generated from the drive share/revoke source contract.",
		"- `passed` requires the configured file to contain the expected pattern.",
		"- Summary counters must match evidence rows.",
		"- Targeted tests must name the share-link and reconciliation checks required by parity.",
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
			? "Drive share/revoke parity evidence is covered for repository cutover gates. Production cutover still requires accepted share-link reconciliation counts."
			: "Drive share/revoke parity evidence is blocked until failed checks pass.",
		"",
		"## Regeneration",
		"",
		"```bash",
		"pnpm check:migration-drive-share-revoke",
		"tools/migration/drive-share-revoke.mjs --write",
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
		command: "tools/migration/drive-share-revoke.mjs --write",
		sources: Object.values(sources),
		targeted_tests: [
			"cargo test -p nvbes-cloud-service share_link --locked",
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
	console.log(`Drive share/revoke evidence written to ${markdownPath} and ${outputPath}`);
	process.exit(0);
}

for (const check of checks) {
	if (check.status === "failed") errors.push(`${check.path}: missing ${check.description}`);
}

for (const [path, expected] of [
	[outputPath, json],
	[markdownPath, markdown],
]) {
	if (!existsSync(path)) errors.push(`${path}: missing; run tools/migration/drive-share-revoke.mjs --write`);
	else if (readFileSync(path, "utf8") !== expected) errors.push(`${path}: stale; run tools/migration/drive-share-revoke.mjs --write`);
}

if (errors.length > 0) {
	console.error("Drive share/revoke evidence checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(`Drive share/revoke evidence: ok (${summary.passed}/${summary.checks} checks passed)`);
