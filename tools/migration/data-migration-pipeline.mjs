#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { readPackageScripts, validateProofCommand } from "./execution-backlog.proof.mjs";

const args = process.argv.slice(2);
const write = args.includes("--write");
const outputPath = "docs/migration/data-migration-pipeline.generated.json";
const markdownPath = "docs/migration/data-migration-pipeline.md";

const sources = {
	dataMap: "docs/migration/data-map.generated.json",
	rejects: "docs/migration/rejects.md",
	reconciliationSchema: "docs/migration/reconciliation-report.schema.json",
	reconciliationTemplate: "docs/migration/reconciliation.template.json",
	reconcileTool: "tools/migration/reconcile.mjs",
	driveUploadDownload: "docs/migration/drive-upload-download.generated.json",
	driveShareRevoke: "docs/migration/drive-share-revoke.generated.json",
	billingEntitlements: "docs/migration/billing-entitlements.generated.json",
	billingWebhookIdempotency: "docs/migration/billing-webhook-idempotency.generated.json",
	billingMultiPspContinuity: "docs/migration/billing-multi-psp-continuity.generated.json",
	releaseFreeze: "docs/migration/release-freeze-manifest.md",
	snapshots: "docs/migration/snapshot-manifest.md",
};

const domainMap = new Map([
	["Identity", ["Identity"]],
	["Workspace", ["Workspace/Authz"]],
	["Drive", ["Drive"]],
	["Billing", ["Billing/Usage"]],
	["Audit", ["Audit/Privacy"]],
	["Privacy", ["Audit/Privacy"]],
	["Developer Platform", ["Developer Platform"]],
]);

const errors = [];
const packageScripts = readPackageScripts(errors);

function read(path) {
	if (!existsSync(path)) {
		errors.push(`${path}: missing`);
		return "";
	}
	return readFileSync(path, "utf8");
}

function readJson(path) {
	const content = read(path);
	if (!content) return undefined;
	try {
		return JSON.parse(content);
	} catch (error) {
		errors.push(`${path}: invalid JSON: ${error.message}`);
		return undefined;
	}
}

function checkText(id, path, description, pattern) {
	const content = read(path);
	return { id, description, path, status: content.includes(pattern) ? "passed" : "failed", pattern };
}

function entriesForDomain(dataMap, domain) {
	const sourceDomains = domainMap.get(domain) ?? [domain];
	return (dataMap.entries ?? []).filter((entry) => sourceDomains.includes(entry.domain));
}

function domainSummary(dataMap, domain) {
	const entries = entriesForDomain(dataMap, domain);
	const reconciliation = new Set(entries.flatMap((entry) => entry.reconciliation ?? []));
	const hasChecksum = entries.some((entry) => entry.reconciliation?.includes("checksum"));
	const hasRejectRules = entries.every((entry) => typeof entry.reject_rule === "string" && entry.reject_rule.length > 0);
	const hasTargets = entries.every((entry) => typeof entry.target === "string" && entry.target.startsWith("target-postgres:"));
	const hasOwners = entries.every((entry) => typeof entry.owner === "string" && !entry.owner.endsWith(" required"));
	const hasTransformDecision = entries.every((entry) => ["rebuild", "replace", "keep", "remove"].includes(entry.decision));
	return {
		domain,
		source_domains: domainMap.get(domain) ?? [domain],
		status:
			entries.length > 0 &&
			reconciliation.has("row_count") &&
			hasChecksum &&
			hasRejectRules &&
			hasTargets &&
			hasOwners &&
			hasTransformDecision
				? "passed"
				: "failed",
		source_tables: entries.length,
		stages: {
			export: entries.length > 0 && reconciliation.has("row_count") ? "passed" : "failed",
			transform: hasTransformDecision && hasTargets ? "passed" : "failed",
			import: hasTargets && hasOwners ? "passed" : "failed",
			reject_log: hasRejectRules ? "passed" : "failed",
			checksum: hasChecksum ? "passed" : "failed",
		},
		reconciliation: [...reconciliation].sort(),
		evidence: [sources.dataMap, sources.rejects, sources.reconciliationSchema, sources.reconcileTool],
	};
}

function buildChecks(dataMap, domains) {
	const checks = [
		checkText("reject-log-columns", sources.rejects, "Reject log includes class, owner, impact, evidence and decision columns", "| Run | Domain | Source | Identifier | Class | Owner | Impact | Evidence | Decision |"),
		checkText("reconcile-schema-domains", sources.reconciliationSchema, "Reconciliation schema requires domain reports", "\"domains\""),
		checkText("reconcile-tool-blocking", sources.reconcileTool, "Reconciliation tool blocks unaccepted row deltas", "row count delta requires accepted status"),
		checkText("reconcile-tool-checksum", sources.reconcileTool, "Reconciliation tool blocks unaccepted checksum mismatches", "checksum mismatch requires accepted status"),
		checkText("reconciliation-template", sources.reconciliationTemplate, "Reconciliation template remains explicit no-go", "\"decision\": \"no-go\""),
		checkText("snapshot-checksum", sources.snapshots, "Snapshot manifest requires checksum evidence", "checksum or"),
		checkText("billing-freeze", sources.releaseFreeze, "Release freeze includes billing mutation freeze", "| billing mutation freeze |"),
		checkText("drive-object-invariant", sources.driveUploadDownload, "Drive upload/download evidence covers object storage invariants", "object"),
		checkText("drive-share-invariant", sources.driveShareRevoke, "Drive share/revoke evidence covers share-link invariants", "share"),
		checkText("billing-ledger-evidence", sources.billingEntitlements, "Billing entitlement evidence is present", "\"status\": \"passed\""), checkText("billing-webhook-idempotency", sources.billingWebhookIdempotency, "Billing webhook idempotency evidence is present", "\"status\": \"passed\""), checkText("billing-multi-psp-continuity", sources.billingMultiPspContinuity, "Billing multi-PSP continuity evidence is present", "\"status\": \"passed\""),
	];
	for (const domain of domains) {
		const summary = domainSummary(dataMap, domain);
		for (const [stage, status] of Object.entries(summary.stages)) {
			checks.push({
				id: `${domain.toLowerCase().replace(/[^a-z0-9]+/g, "-")}-${stage}`,
				description: `${domain} data migration ${stage.replace("_", " ")} contract`,
				path: sources.dataMap,
				status,
				pattern: `${summary.source_tables} source table(s), ${summary.reconciliation.join(", ")}`,
			});
		}
	}
	return checks;
}

function summarize(checks, domains) {
	const failed = checks.filter((check) => check.status === "failed").length;
	const failedDomains = domains.filter((domain) => domain.status === "failed").length;
	return {
		checks: checks.length,
		passed: checks.length - failed,
		failed,
		domains: domains.length,
		domains_passed: domains.length - failedDomains,
		domains_failed: failedDomains,
		status: failed === 0 && failedDomains === 0 ? "passed" : "failed",
	};
}

const sameItems = (actual, expected) => Array.isArray(actual) && actual.length === expected.length && actual.every((item, index) => item === expected[index]);

function validateReport(report) {
	const expectedDomains = [...domainMap.keys()];
	const domainSeen = new Set();
	for (const domain of report.domains) {
		if (domainSeen.has(domain.domain)) errors.push(`${outputPath}: duplicate domain ${domain.domain}`);
		domainSeen.add(domain.domain);
		if (!expectedDomains.includes(domain.domain)) errors.push(`${outputPath}: stale domain ${domain.domain}`);
		if (!["passed", "failed"].includes(domain.status)) errors.push(`${domain.domain}: unsupported status ${domain.status}`);
		if (!sameItems(domain.source_domains, domainMap.get(domain.domain) ?? [domain.domain])) {
			errors.push(`${domain.domain}: source_domains must match data migration contract`);
		}
		if (!Array.isArray(domain.evidence) || domain.evidence.length === 0) errors.push(`${domain.domain}: evidence is required`);
	}
	for (const domain of expectedDomains) {
		if (!domainSeen.has(domain)) errors.push(`${outputPath}: missing domain ${domain}`);
	}
	const checkSeen = new Set();
	for (const check of report.checks) {
		if (checkSeen.has(check.id)) errors.push(`${outputPath}: duplicate check ${check.id}`);
		checkSeen.add(check.id);
		if (!check.description) errors.push(`${check.id}: description is required`);
		if (!Object.values(sources).includes(check.path)) errors.push(`${check.id}: path is not in data migration source contract`);
		if (!["passed", "failed"].includes(check.status)) errors.push(`${check.id}: unsupported status ${check.status}`);
		if (!check.pattern) errors.push(`${check.id}: pattern is required`);
	}
	const expectedSummary = summarize(report.checks, report.domains);
	for (const [field, value] of Object.entries(expectedSummary)) {
		if (report.summary[field] !== value) errors.push(`${outputPath}: summary.${field} must be ${value}`);
	}
	if (report.schema_version !== 1) errors.push(`${outputPath}: schema_version must be 1`);
	if (report.generation?.command !== "node tools/migration/data-migration-pipeline.mjs --write") {
		errors.push(`${outputPath}: generation.command is invalid`);
	}
	if (!sameItems(report.generation?.sources, Object.values(sources))) {
		errors.push(`${outputPath}: generation.sources must match data migration source contract`);
	}
	if (!sameItems(report.generation?.targeted_tests, ["pnpm check:migration-data-map", "pnpm check:migration-rejects", "pnpm check:migration-reconciliation-report", "pnpm check:migration-reconciliation-template"])) {
		errors.push(`${outputPath}: generation.targeted_tests is invalid`);
	}
	for (const command of report.generation?.targeted_tests ?? []) {
		errors.push(...validateProofCommand({ id: "data-migration-pipeline", proof: command }, packageScripts));
	}
}

const serializeJson = (data) => `${JSON.stringify(data, null, 2)}\n`;

function serializeMarkdown(data) {
	const lines = [
		"# Data Migration Pipeline Evidence",
		"",
		"## Status",
		"",
		`- status: ${data.summary.status}`,
		`- domains: ${data.summary.domains}`,
		`- domains_passed: ${data.summary.domains_passed}`,
		`- checks: ${data.summary.checks}`,
		`- passed: ${data.summary.passed}`,
		`- failed: ${data.summary.failed}`,
		"",
		"## Rules",
		"",
		"- Every domain row must match the data migration domain contract.",
		"- Every evidence row must be generated from the data migration source contract.",
		"- Summary counters must match domain and evidence rows.",
		"- Production cutover still requires executed reconciliation reports, not template evidence.",
		"- Generation provenance must identify sources, write command and targeted tests.",
		"",
		"## Domains",
		"",
		"| Domain | Status | Source tables | Reconciliation |",
		"|---|---:|---:|---|",
	];
	for (const domain of data.domains) {
		lines.push(`| ${domain.domain} | ${domain.status} | ${domain.source_tables} | ${domain.reconciliation.join(", ")} |`);
	}
	lines.push("", "## Evidence", "", "| Check | Status | Path |", "|---|---:|---|");
	for (const check of data.checks) {
		lines.push(`| ${check.description} | ${check.status} | \`${check.path}\` |`);
	}
	lines.push(
		"",
		"## Decision",
		"",
		data.summary.status === "passed"
			? "Data migration pipeline repository evidence is covered for export, transform, import, reject logs and checksums. Production cutover still requires executed reconciliation reports with accepted counts and checksums."
			: "Data migration pipeline evidence is blocked until failed checks pass.",
		"",
		"## Regeneration",
		"",
		"```bash",
		"pnpm check:migration-data-migration-pipeline",
		"node tools/migration/data-migration-pipeline.mjs --write",
		"```",
		"",
	);
	return lines.join("\n");
}

const dataMap = readJson(sources.dataMap) ?? { entries: [] };
const domainNames = [...domainMap.keys()];
const domains = domainNames.map((domain) => domainSummary(dataMap, domain));
const checks = buildChecks(dataMap, domainNames);
const summary = summarize(checks, domains);
const report = {
	schema_version: 1,
	generation: {
		command: "node tools/migration/data-migration-pipeline.mjs --write",
		sources: Object.values(sources),
		targeted_tests: [
			"pnpm check:migration-data-map",
			"pnpm check:migration-rejects",
			"pnpm check:migration-reconciliation-report",
			"pnpm check:migration-reconciliation-template",
		],
	},
	summary,
	domains,
	checks,
};

validateReport(report);

const json = serializeJson(report);
const markdown = serializeMarkdown(report);

if (write) {
	mkdirSync(dirname(outputPath), { recursive: true });
	writeFileSync(outputPath, json);
	writeFileSync(markdownPath, markdown);
	console.log(`Data migration pipeline evidence written to ${markdownPath} and ${outputPath}`);
	process.exit(0);
}

for (const domain of domains) {
	if (domain.status === "failed") errors.push(`${domain.domain}: data migration pipeline contract is incomplete`);
}
for (const check of checks) {
	if (check.status === "failed") errors.push(`${check.path}: missing ${check.description}`);
}
for (const [path, expected] of [
	[outputPath, json],
	[markdownPath, markdown],
]) {
	if (!existsSync(path)) errors.push(`${path}: missing; run node tools/migration/data-migration-pipeline.mjs --write`);
	else if (readFileSync(path, "utf8") !== expected) errors.push(`${path}: stale; run node tools/migration/data-migration-pipeline.mjs --write`);
}

if (errors.length > 0) {
	console.error("Data migration pipeline evidence checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(`Data migration pipeline evidence: ok (${summary.domains_passed}/${summary.domains} domains, ${summary.passed}/${summary.checks} checks passed)`);
