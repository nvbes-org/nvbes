#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";

const args = process.argv.slice(2);
const write = args.includes("--write");
const outputPath = "docs/migration/audit-append-only.generated.json";
const markdownPath = "docs/migration/audit-append-only.md";
const auditCratePath = "libs/rust/audit/src/lib.rs";

const migrationTargets = [
	{
		product: "identity",
		path: "apps/identity-service/migrations/0001_initial_schema.sql",
		partition: "tenant_id = NEW.tenant_id",
	},
	{
		product: "drive",
		path: "apps/cloud-service/migrations/0001_initial_schema.sql",
		partition: "workspace_id = NEW.workspace_id",
	},
];

const errors = [];

function read(path) {
	if (!existsSync(path)) {
		errors.push(`${path}: missing`);
		return "";
	}
	return readFileSync(path, "utf8");
}

function includesAll(source, path, checks) {
	return checks.map((check) => ({
		check: check.label,
		status: source.includes(check.pattern) ? "passed" : "failed",
		pattern: check.pattern,
		path,
	}));
}

function migrationChecks(target) {
	const sql = read(target.path);
	return includesAll(sql, target.path, [
		{ label: "pgcrypto digest extension", pattern: "CREATE EXTENSION IF NOT EXISTS pgcrypto" },
		{ label: "hash chain column", pattern: "previous_event_hash" },
		{ label: "event hash column", pattern: "event_hash" },
		{ label: "hash chain function", pattern: "CREATE OR REPLACE FUNCTION audit_events_set_hash()" },
		{ label: "hash partition", pattern: target.partition },
		{ label: "sha256 digest", pattern: "'sha256'" },
		{ label: "previous hash included", pattern: "COALESCE(previous_hash, '')" },
		{ label: "insert hash trigger", pattern: "CREATE TRIGGER trg_audit_events_set_hash" },
		{ label: "insert trigger timing", pattern: "BEFORE INSERT ON audit_events" },
		{ label: "mutation blocker function", pattern: "CREATE OR REPLACE FUNCTION audit_events_prevent_mutation()" },
		{ label: "update blocker trigger", pattern: "CREATE TRIGGER trg_audit_events_prevent_update" },
		{ label: "update trigger timing", pattern: "BEFORE UPDATE ON audit_events" },
		{ label: "delete blocker trigger", pattern: "CREATE TRIGGER trg_audit_events_prevent_delete" },
		{ label: "delete trigger timing", pattern: "BEFORE DELETE ON audit_events" },
		{ label: "unique event hash index", pattern: "idx_audit_events_event_hash" },
	]).map((check) => ({ product: target.product, ...check }));
}

function auditCrateChecks() {
	const rust = read(auditCratePath);
	const checks = includesAll(rust, auditCratePath, [
		{ label: "pool insert API", pattern: "pub async fn insert_audit_event_pool" },
		{ label: "transaction insert API", pattern: "pub async fn insert_audit_event_tx" },
		{ label: "event hash delegated to trigger", pattern: "Le trigger SQL se charge de calculer le chaînage de hash" },
	]).map((check) => ({ product: "shared-audit-crate", ...check }));

	const insertColumns = [...rust.matchAll(/INSERT INTO audit_events\s*\(([\s\S]*?)\)\s*VALUES/g)].map((match) => match[1]);
	checks.push({
		product: "shared-audit-crate",
		check: "event hash omitted from insert columns",
		status: insertColumns.length >= 2 && insertColumns.every((columns) => !columns.includes("event_hash")) ? "passed" : "failed",
		pattern: "audit insert column lists do not include event_hash",
		path: auditCratePath,
	});

	return checks;
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
	const sources = [...migrationTargets.map((target) => target.path), auditCratePath];
	const seen = new Set();
	for (const check of report.checks) {
		const key = `${check.product}:${check.check}:${check.path}`;
		if (seen.has(key)) errors.push(`${outputPath}: duplicate check ${key}`);
		seen.add(key);
		if (!check.product) errors.push(`${outputPath}: check product is required`);
		if (!check.check) errors.push(`${outputPath}: check label is required`);
		if (!sources.includes(check.path)) errors.push(`${key}: path is not in audit append-only source contract`);
		if (!["passed", "failed"].includes(check.status)) errors.push(`${key}: unsupported status ${check.status}`);
		if (!check.pattern) errors.push(`${key}: pattern is required`);
	}
	const expectedSummary = summarize(report.checks);
	for (const [field, value] of Object.entries(expectedSummary)) {
		if (report.summary[field] !== value) errors.push(`${outputPath}: summary.${field} must be ${value}`);
	}
	if (report.schema_version !== 1) errors.push(`${outputPath}: schema_version must be 1`);
	if (report.generation?.command !== "node tools/migration/audit-append-only.mjs --write") {
		errors.push(`${outputPath}: generation.command is invalid`);
	}
	if (!sameItems(report.generation?.sources, sources)) {
		errors.push(`${outputPath}: generation.sources must match audit append-only source contract`);
	}
}

function serializeJson(data) {
	return `${JSON.stringify(data, null, 2)}\n`;
}

function serializeMarkdown(data) {
	const lines = [
		"# Audit Append-Only Evidence",
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
		"- Every evidence row must be generated from the audit append-only source contract.",
		"- `passed` requires the configured migration or shared audit crate file to contain the expected pattern.",
		"- Summary counters must match evidence rows.",
		"- The generated source list must include both product migrations and the shared audit crate.",
		"- Generation provenance must identify sources and write command.",
		"",
		"## Evidence",
		"",
		"| Product | Check | Status | Path |",
		"|---|---|---:|---|",
	];
	for (const check of data.checks) {
		lines.push(`| ${check.product} | ${check.check} | ${check.status} | \`${check.path}\` |`);
	}
	lines.push(
		"",
		"## Decision",
		"",
		data.summary.failed === 0
			? "Audit append-only parity evidence is covered for repository cutover gates."
			: "Audit append-only parity evidence is blocked until failed checks pass.",
		"",
		"## Regeneration",
		"",
		"```bash",
		"pnpm check:migration-audit-append-only",
		"node tools/migration/audit-append-only.mjs --write",
		"```",
		"",
	);
	return lines.join("\n");
}

const checks = [...migrationTargets.flatMap(migrationChecks), ...auditCrateChecks()];
const summary = summarize(checks);
const report = {
	schema_version: 1,
	generation: {
		command: "node tools/migration/audit-append-only.mjs --write",
		sources: [...migrationTargets.map((target) => target.path), auditCratePath],
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
	console.log(`Audit append-only evidence written to ${markdownPath} and ${outputPath}`);
	process.exit(0);
}

for (const check of checks) {
	if (check.status === "failed") errors.push(`${check.path}: missing ${check.check}`);
}

for (const [path, expected] of [
	[outputPath, json],
	[markdownPath, markdown],
]) {
	if (!existsSync(path)) errors.push(`${path}: missing; run node tools/migration/audit-append-only.mjs --write`);
	else if (readFileSync(path, "utf8") !== expected) errors.push(`${path}: stale; run node tools/migration/audit-append-only.mjs --write`);
}

if (errors.length > 0) {
	console.error("Audit append-only evidence checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(`Audit append-only evidence: ok (${summary.passed}/${summary.checks} checks passed)`);
