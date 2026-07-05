#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { validateDecisionMapRows } from "./decision-map.validation.mjs";

const inventoryPath = "docs/migration/inventory.generated.json";
const outputPath = "docs/migration/data-map.generated.json";
const markdownPath = "docs/migration/data-map.md";
const args = process.argv.slice(2);
const write = args.includes("--write");
const strict = args.includes("--strict");
const errors = [];

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

function keyFor(entry) {
	return `${entry.source.file}#${entry.source.table}`;
}

function inferDomain(file, table) {
	const name = table.toLowerCase();
	if (file.startsWith("apps/billing-api/migrations/")) return "Billing/Usage";
	if (name.includes("developer") || name.includes("oauth_client") || name.includes("webhook")) return "Developer Platform";
	if (name.includes("billing") || name.includes("invoice") || name.includes("subscription") || name.includes("usage") || name.includes("plan") || name.includes("stripe")) return "Billing/Usage";
	if (name.includes("audit") || name.includes("privacy") || name.includes("consent")) return "Audit/Privacy";
	if (name.includes("storage") || name.includes("upload") || name.includes("share") || name.includes("quota")) return "Drive";
	if (name.includes("workspace") || name.includes("organization") || name.includes("tenant") || name.includes("member") || name.includes("invitation")) return "Workspace/Authz";
	if (file.includes("drive-api/")) return "Drive";
	return "Identity";
}

function inferClassification(table) {
	const name = table.toLowerCase();
	if (name.includes("developer") || name.includes("webhook")) return "operational data";
	if (name.includes("billing") || name.includes("invoice") || name.includes("subscription") || name.includes("usage")) return "financial data";
	if (name.includes("password") || name.includes("mfa") || name.includes("session") || name.includes("token") || name.includes("key") || name.includes("secret")) return "sensitive personal data";
	if (name.includes("audit") || name.includes("risk")) return "audit data";
	if (name.includes("user") || name.includes("member") || name.includes("tenant") || name.includes("workspace") || name.includes("email")) return "personal data";
	return "operational data";
}

function defaultReconciliation(table) {
	const checks = ["row_count"];
	const name = table.toLowerCase();
	if (!name.includes("token") && !name.includes("secret")) checks.push("checksum");
	if (name.includes("member") || name.includes("workspace") || name.includes("tenant") || name.includes("user")) checks.push("orphan_check");
	if (name.includes("storage") || name.includes("upload") || name.includes("share")) checks.push("object_or_link_invariant");
	if (!name.includes("developer") && !name.includes("webhook") && (name.includes("billing") || name.includes("invoice") || name.includes("usage") || name.includes("subscription"))) checks.push("ledger_balance");
	return checks;
}

function slug(value) {
	return value.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "");
}

function targetFor(domain, table) {
	return `target-postgres:${slug(domain)}.${table}`;
}

function ownerFor(domain) {
	if (domain === "Billing/Usage") return "Billing lead";
	if (domain === "Audit/Privacy") return "Privacy lead";
	if (domain === "Drive") return "Drive lead";
	if (domain === "Workspace/Authz") return "Workspace lead";
	if (domain === "Developer Platform") return "Developer Platform lead";
	if (domain === "Identity") return "Identity lead";
	return "Data lead";
}

function retentionFor(classification, table) {
	const name = table.toLowerCase();
	if (classification === "financial data") return "contractual-financial-retention";
	if (classification === "audit data") return "append-only-audit-retention";
	if (classification === "sensitive personal data") return "shortest-legal-retention";
	if (classification === "personal data") return "product-lifecycle-plus-privacy-retention";
	if (name.includes("token") || name.includes("session") || name.includes("nonce")) return "ephemeral-security-retention";
	return "operational-retention";
}

function rejectRuleFor(classification) {
	if (classification === "financial data") return "blocking unless billing owner accepts ledger exception";
	if (classification === "audit data") return "blocking unless audit owner signs append-only gap";
	if (classification.includes("personal")) return "blocking unless privacy owner accepts subject-impact exception";
	return "blocking until domain owner signs reject disposition";
}

function sourceEntries(inventory) {
	const entries = [];
	for (const fileEntry of inventory.tables ?? []) {
		for (const table of fileEntry.tables ?? []) {
			const domain = inferDomain(fileEntry.file, table);
			const classification = inferClassification(table);
			entries.push({
				source: { file: fileEntry.file, table },
				domain,
				decision: "rebuild",
				target: targetFor(domain, table),
				owner: ownerFor(domain),
				classification,
				retention: retentionFor(classification, table),
				reconciliation: defaultReconciliation(table),
				reject_rule: rejectRuleFor(classification),
			});
		}
	}
	return entries.sort((a, b) => keyFor(a).localeCompare(keyFor(b)));
}

function mergeExisting(generated, existing) {
	if (!existing?.entries) return generated;
	const byKey = new Map(existing.entries.map((entry) => [keyFor(entry), entry]));
	return generated.map((entry) => {
		const current = byKey.get(keyFor(entry));
		if (!current) return entry;
		if (current.domain && current.domain !== entry.domain) {
			return {
				...entry,
				decision: current.decision && current.decision !== "pending" ? current.decision : entry.decision,
			};
		}
		const expectedTargetPrefix = `target-postgres:${slug(entry.domain)}.`;
		if (current.target?.startsWith("target-postgres:") && !current.target.startsWith(expectedTargetPrefix)) {
			return {
				...entry,
				decision: current.decision && current.decision !== "pending" ? current.decision : entry.decision,
			};
		}
		if (entry.source.file.startsWith("apps/billing-api/migrations/")) {
			return {
				...entry,
				decision: current.decision && current.decision !== "pending" ? current.decision : entry.decision,
			};
		}
		return {
			...entry,
			decision: current.decision && current.decision !== "pending" ? current.decision : entry.decision,
			target: current.target && !current.target.startsWith("pending:") ? current.target : entry.target,
			owner: current.owner && current.owner !== "migration lead required" ? current.owner : entry.owner,
			classification: current.classification ?? entry.classification,
			retention: current.retention && current.retention !== "pending" ? current.retention : entry.retention,
			reconciliation: current.reconciliation ?? entry.reconciliation,
			reject_rule: current.reject_rule && !current.reject_rule.startsWith("blocking until")
				? current.reject_rule
				: entry.reject_rule,
			source: entry.source,
		};
	});
}

function serialize(data) {
	const lines = [
		"{",
		`  "schema_version": ${JSON.stringify(data.schema_version)},`,
		`  "generation": ${JSON.stringify(data.generation)},`,
		`  "summary": ${JSON.stringify(data.summary)},`,
		'  "entries": [',
	];
	for (const [index, entry] of data.entries.entries()) {
		const suffix = index === data.entries.length - 1 ? "" : ",";
		lines.push(`    ${JSON.stringify(entry)}${suffix}`);
	}
	lines.push("  ]", "}");
	return `${lines.join("\n")}\n`;
}

function serializeMarkdown(data) {
	const lines = [
		"# Data Migration Map",
		"",
		"## Status",
		"",
		`- entries: ${data.summary.entries}`,
		`- pending: ${data.summary.pending}`,
		`- keep: ${data.summary.keep}`,
		`- rebuild: ${data.summary.rebuild}`,
		`- remove: ${data.summary.remove}`,
		`- replace: ${data.summary.replace}`,
		"",
		"## Rules",
		"",
		"- Every source table maps to a target table, reject rule, or deletion rule.",
		"- Critical data requires row counts and checksums; financial data also requires ledger balance.",
		"- Personal data carries classification, retention, and owner decisions.",
		"- Source rows, domains and summary counters must match the inventory-derived map.",
		"- Production cutover still requires a separate reconciliation report with accepted checksums.",
		"- Generation provenance must identify source, write command and strict cutover command.",
		"",
		"## Table Map",
		"",
		"| Source | Target | Domain | Classification | Retention | Reconciliation | Status |",
		"|---|---|---|---|---|---|---|",
	];
	for (const entry of data.entries) {
		lines.push(
			`| ${entry.source.file}#${entry.source.table} | \`${entry.target}\` | ${entry.domain} | ${entry.classification} | ${entry.retention} | ${entry.reconciliation.join(", ")} | ${entry.decision} |`,
		);
	}
	lines.push(
		"",
		"## Reject Classes",
		"",
		"| Class | Description | Cutover impact |",
		"|---|---|---|",
		"| fixed | corrected before cutover | none |",
		"| accepted | owner accepts migration gap | allowed only with evidence |",
		"| rejected | excluded from migration | requires user/legal impact review |",
		"| blocking | must stop cutover | no-go |",
		"",
		"## Regeneration",
		"",
		"```bash",
		"pnpm check:migration-data-map",
		"tools/migration/data-map.mjs --write",
		"```",
		"",
	);
	return lines.join("\n");
}

function validate(map, expectedEntries) {
	const expectedKeys = new Set(expectedEntries.map(keyFor));
	const seen = new Set();
	if (map.schema_version !== 1) errors.push(`${outputPath}: schema_version must be 1`);
	if (map.generation?.command !== "tools/migration/data-map.mjs --write") errors.push(`${outputPath}: generation.command is invalid`);
	if (map.generation?.source !== inventoryPath) errors.push(`${outputPath}: generation.source is invalid`);
	if (map.generation?.strict_cutover_command !== "tools/migration/data-map.mjs --strict") errors.push(`${outputPath}: generation.strict_cutover_command is invalid`);
	if (!Array.isArray(map.entries)) errors.push(`${outputPath}: entries must be an array`);
	validateDecisionMapRows({ map, expectedEntries, keyFor, outputPath, errors });
	for (const entry of map.entries ?? []) {
		const key = keyFor(entry);
		if (seen.has(key)) errors.push(`${outputPath}: duplicate entry ${key}`);
		seen.add(key);
		if (!expectedKeys.has(key)) errors.push(`${outputPath}: stale entry ${key}`);
		if (!entry.domain) errors.push(`${key}: domain is required`);
		if (!["pending", "keep", "rebuild", "remove", "replace"].includes(entry.decision)) errors.push(`${key}: unsupported decision ${entry.decision}`);
		if (!entry.target) errors.push(`${key}: target is required`);
		if (!entry.owner) errors.push(`${key}: owner is required`);
		if (!entry.classification) errors.push(`${key}: classification is required`);
		if (!entry.retention) errors.push(`${key}: retention is required`);
		if (!Array.isArray(entry.reconciliation) || entry.reconciliation.length === 0) errors.push(`${key}: reconciliation checks are required`);
		if (!entry.reject_rule) errors.push(`${key}: reject_rule is required`);
		if (strict) {
			if (entry.decision === "pending") errors.push(`${key}: pending decision blocks cutover`);
			if (entry.owner === "migration lead required") errors.push(`${key}: owner must be assigned`);
			if (entry.target.startsWith("pending:")) errors.push(`${key}: target must be resolved`);
		}
	}
	for (const key of expectedKeys) if (!seen.has(key)) errors.push(`${outputPath}: missing entry ${key}`);
}

const inventory = readJson(inventoryPath);
const generatedEntries = inventory ? sourceEntries(inventory) : [];
const existing = existsSync(outputPath) ? readJson(outputPath) : undefined;
const dataMap = {
	schema_version: 1,
	generation: {
		command: "tools/migration/data-map.mjs --write",
		source: inventoryPath,
		strict_cutover_command: "tools/migration/data-map.mjs --strict",
	},
	summary: { entries: generatedEntries.length, pending: 0, keep: 0, rebuild: 0, remove: 0, replace: 0 },
	entries: mergeExisting(generatedEntries, existing),
};

for (const entry of dataMap.entries) dataMap.summary[entry.decision] += 1;

if (write) {
	mkdirSync(dirname(outputPath), { recursive: true });
	writeFileSync(outputPath, serialize(dataMap));
	writeFileSync(markdownPath, serializeMarkdown(dataMap));
	console.log(`Migration data map written to ${markdownPath} and ${outputPath}`);
	process.exit(0);
}

validate(dataMap, generatedEntries);

if (!existsSync(outputPath)) errors.push(`${outputPath}: missing; run tools/migration/data-map.mjs --write`);
else if (readFileSync(outputPath, "utf8") !== serialize(dataMap)) errors.push(`${outputPath}: stale; run tools/migration/data-map.mjs --write`);
if (!existsSync(markdownPath)) errors.push(`${markdownPath}: missing; run tools/migration/data-map.mjs --write`);
else if (readFileSync(markdownPath, "utf8") !== serializeMarkdown(dataMap)) errors.push(`${markdownPath}: stale; run tools/migration/data-map.mjs --write`);

if (errors.length > 0) {
	console.error("Migration data-map checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(`Migration data map: ok (${dataMap.summary.entries} entries, ${dataMap.summary.pending} pending)`);
