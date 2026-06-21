#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { validateDecisionMapRows } from "./decision-map.validation.mjs";

const inventoryPath = "docs/migration/inventory.generated.json";
const outputPath = "docs/migration/job-map.generated.json";
const markdownPath = "docs/migration/job-map.md";
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
	return `${entry.source.file}#${entry.source.symbol}`;
}

function parseConstant(value) {
	const [symbol, job_type] = value.split("=");
	return { symbol, job_type };
}

function inferDomain(jobType) {
	if (jobType.includes("billing") || jobType.includes("stripe")) return "Billing/Usage";
	if (jobType.includes("privacy") || jobType.includes("data.export")) return "Audit/Privacy";
	if (jobType.includes("email")) return "Email";
	if (jobType.includes("storage") || jobType.includes("uploads") || jobType.includes("trash") || jobType.includes("quotas")) return "Drive";
	return "Platform";
}

function defaultVerification(jobType) {
	const checks = ["handler_exists", "retry_policy", "idempotency_key"];
	if (jobType.includes("billing") || jobType.includes("stripe")) checks.push("webhook_replay");
	if (jobType.includes("privacy") || jobType.includes("data.export")) checks.push("privacy_smoke");
	if (jobType.includes("storage") || jobType.includes("uploads")) checks.push("storage_reconciliation");
	return checks;
}

function slug(value) {
	return value.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "");
}

function targetFor(jobType) {
	return `apps/workers#${slug(jobType)}`;
}

function replacementFor(jobType) {
	return `target-worker:${jobType}`;
}

function ownerFor(jobType) {
	if (jobType.includes("billing") || jobType.includes("stripe")) return "Billing lead";
	if (jobType.includes("privacy") || jobType.includes("data.export")) return "Privacy lead";
	if (jobType.includes("email")) return "Platform lead";
	if (jobType.includes("storage") || jobType.includes("uploads") || jobType.includes("trash") || jobType.includes("quotas")) return "Drive lead";
	return "Infra lead";
}

function cutoverHandlingFor(jobType) {
	if (jobType.includes("billing") || jobType.includes("stripe")) {
		return "rebuild in target worker runtime; block cutover until webhook replay and ledger reconciliation pass";
	}
	if (jobType.includes("privacy") || jobType.includes("data.export")) {
		return "rebuild in target worker runtime; block cutover until export/delete smoke tests and reject handling pass";
	}
	if (jobType.includes("storage") || jobType.includes("uploads")) {
		return "rebuild in target worker runtime; block cutover until storage reconciliation and retry evidence pass";
	}
	return "rebuild in target worker runtime; block cutover until handler, retry and idempotency evidence pass";
}

function sourceEntries(inventory) {
	const seen = new Set();
	const entries = [];
	for (const fileEntry of inventory.jobs ?? []) {
		for (const constant of fileEntry.constants ?? []) {
			const parsed = parseConstant(constant);
			const source = { file: fileEntry.file, symbol: parsed.symbol, job_type: parsed.job_type };
			const key = `${source.file}#${source.symbol}`;
			if (seen.has(key)) continue;
			seen.add(key);
			entries.push({
				source,
				domain: inferDomain(parsed.job_type),
				decision: "rebuild",
				target: targetFor(parsed.job_type),
				owner: ownerFor(parsed.job_type),
				replacement: replacementFor(parsed.job_type),
				criticality: "critical",
				cutover_handling: cutoverHandlingFor(parsed.job_type),
				verification: defaultVerification(parsed.job_type),
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
		return {
			...entry,
			decision: current.decision && current.decision !== "pending" ? current.decision : entry.decision,
			target: current.target && !current.target.startsWith("pending:") ? current.target : entry.target,
			owner: current.owner && current.owner !== "migration lead required" ? current.owner : entry.owner,
			replacement: current.replacement && current.replacement !== "pending" ? current.replacement : entry.replacement,
			criticality: current.criticality ?? entry.criticality,
			cutover_handling: current.cutover_handling && !current.cutover_handling.startsWith("blocking until")
				? current.cutover_handling
				: entry.cutover_handling,
			verification: current.verification ?? entry.verification,
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
		"# Job Migration Map",
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
		"- Every job must have a target worker, replacement path, owner and verification checks.",
		"- Source rows, domains and summary counters must match the inventory-derived map.",
		"- Generation provenance must identify source, write command and strict cutover command.",
		"",
		"## Jobs",
		"",
		"| Job | Domain | Decision | Owner | Target | Replacement |",
		"|---|---|---:|---|---|---|",
	];
	for (const entry of data.entries) {
		lines.push(
			`| ${entry.source.job_type} | ${entry.domain} | ${entry.decision} | ${entry.owner} | \`${entry.target}\` | \`${entry.replacement}\` |`,
		);
	}
	lines.push(
		"",
		"## Regeneration",
		"",
		"```bash",
		"pnpm check:migration-job-map",
		"tools/migration/job-map.mjs --write",
		"```",
		"",
	);
	return lines.join("\n");
}

function validate(map, expectedEntries) {
	const expectedKeys = new Set(expectedEntries.map(keyFor));
	const seen = new Set();
	if (map.schema_version !== 1) errors.push(`${outputPath}: schema_version must be 1`);
	if (map.generation?.command !== "tools/migration/job-map.mjs --write") errors.push(`${outputPath}: generation.command is invalid`);
	if (map.generation?.source !== inventoryPath) errors.push(`${outputPath}: generation.source is invalid`);
	if (map.generation?.strict_cutover_command !== "tools/migration/job-map.mjs --strict") errors.push(`${outputPath}: generation.strict_cutover_command is invalid`);
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
		if (!entry.replacement) errors.push(`${key}: replacement is required`);
		if (!entry.criticality) errors.push(`${key}: criticality is required`);
		if (!entry.cutover_handling) errors.push(`${key}: cutover_handling is required`);
		if (!Array.isArray(entry.verification) || entry.verification.length === 0) errors.push(`${key}: verification checks are required`);
		if (strict) {
			if (entry.decision === "pending") errors.push(`${key}: pending decision blocks cutover`);
			if (entry.owner === "migration lead required") errors.push(`${key}: owner must be assigned`);
			if (entry.target.startsWith("pending:")) errors.push(`${key}: target must be resolved`);
			if (entry.replacement === "pending") errors.push(`${key}: replacement must be resolved`);
		}
	}
	for (const key of expectedKeys) {
		if (!seen.has(key)) errors.push(`${outputPath}: missing entry ${key}`);
	}
}

const inventory = readJson(inventoryPath);
const generatedEntries = inventory ? sourceEntries(inventory) : [];
const existing = existsSync(outputPath) ? readJson(outputPath) : undefined;
const jobMap = {
	schema_version: 1,
	generation: {
		command: "tools/migration/job-map.mjs --write",
		source: inventoryPath,
		strict_cutover_command: "tools/migration/job-map.mjs --strict",
	},
	summary: { entries: generatedEntries.length, pending: 0, keep: 0, rebuild: 0, remove: 0, replace: 0 },
	entries: mergeExisting(generatedEntries, existing),
};

for (const entry of jobMap.entries) {
	jobMap.summary[entry.decision] += 1;
}

if (write) {
	mkdirSync(dirname(outputPath), { recursive: true });
	writeFileSync(outputPath, serialize(jobMap));
	writeFileSync(markdownPath, serializeMarkdown(jobMap));
	console.log(`Migration job map written to ${markdownPath} and ${outputPath}`);
	process.exit(0);
}

validate(jobMap, generatedEntries);

if (!existsSync(outputPath)) {
	errors.push(`${outputPath}: missing; run tools/migration/job-map.mjs --write`);
} else if (readFileSync(outputPath, "utf8") !== serialize(jobMap)) {
	errors.push(`${outputPath}: stale; run tools/migration/job-map.mjs --write`);
}

if (!existsSync(markdownPath)) {
	errors.push(`${markdownPath}: missing; run tools/migration/job-map.mjs --write`);
} else if (readFileSync(markdownPath, "utf8") !== serializeMarkdown(jobMap)) {
	errors.push(`${markdownPath}: stale; run tools/migration/job-map.mjs --write`);
}

if (errors.length > 0) {
	console.error("Migration job-map checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(`Migration job map: ok (${jobMap.summary.entries} entries, ${jobMap.summary.pending} pending)`);
