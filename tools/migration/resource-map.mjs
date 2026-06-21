#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { validateDecisionMapRows } from "./decision-map.validation.mjs";

const inventoryPath = "docs/migration/inventory.generated.json";
const outputPath = "docs/migration/resource-map.generated.json";
const markdownPath = "docs/migration/resource-map.md";
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
	return `${entry.source.type}:${entry.source.name}:${entry.source.source}`;
}

function inferDomain(resource) {
	if (resource.type === "bucket") return "Drive";
	if (resource.type === "queue" && resource.name.includes("billing")) return "Billing/Usage";
	if (resource.type === "queue" && (resource.name.includes("privacy") || resource.name === "data.export")) return "Audit/Privacy";
	if (resource.type === "queue" && resource.name.includes("email")) return "Email";
	if (resource.type === "queue") return "Drive";
	if (resource.name.startsWith("billing.")) return "Billing/Usage";
	if (resource.name.startsWith("drive.")) return "Drive";
	if (resource.name.startsWith("workspace.")) return "Workspace/Authz";
	if (resource.name.startsWith("identity.")) return "Identity";
	return "Platform";
}

function defaultVerification(resource) {
	if (resource.type === "bucket") return ["private_policy", "versioning_or_backup_decision", "object_reconciliation"];
	if (resource.type === "queue") return ["consumer_exists", "retry_policy", "dlq_or_replay_decision"];
	if (resource.type === "event_topic") return ["schema_exists", "outbox_publication", "consumer_idempotency"];
	return ["owner_signoff"];
}

function slug(value) {
	return value.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "");
}

function inferDecision(resource) {
	if (resource.type === "bucket") return "replace";
	if (resource.type === "event_topic") return "keep";
	if (resource.type === "queue") return "rebuild";
	return "replace";
}

function inferTarget(resource, domain) {
	if (resource.type === "bucket") return "deploy/oss/opentofu#s3-compatible-files-bucket";
	if (resource.type === "event_topic") return resource.schema ?? `contracts/events#${resource.name}`;
	if (resource.type === "queue") return `apps/workers#${slug(resource.name)}`;
	return `deploy/oss#${slug(resource.name)}`;
}

function inferOwner(resource) {
	if (resource.type === "bucket") return "Data lead";
	if (resource.type === "event_topic") return "Platform lead";
	if (resource.type === "queue") return "Infra lead";
	return "Migration lead";
}

function inferCutoverHandling(resource) {
	if (resource.type === "bucket") {
		return "replace Scaleway bucket with S3-compatible target; block cutover until object reconciliation and backup restore pass";
	}
	if (resource.type === "event_topic") {
		return "keep versioned contract and bind it to the target outbox/schema-registry pipeline";
	}
	if (resource.type === "queue") {
		return "rebuild legacy queue as target worker capability with retry policy and DLQ or replay decision";
	}
	return "resolve target resource before cutover";
}

function sourceEntries(inventory) {
	return (inventory.resources ?? [])
		.map((resource) => {
			const domain = inferDomain(resource);
			return {
				source: resource,
				domain,
				decision: inferDecision(resource),
				target: inferTarget(resource, domain),
				owner: inferOwner(resource),
				classification: resource.critical ? "critical" : "operational",
				cutover_handling: inferCutoverHandling(resource),
				verification: defaultVerification(resource),
			};
		})
		.sort((a, b) => keyFor(a).localeCompare(keyFor(b)));
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
			owner: current.owner && !current.owner.endsWith(" required") ? current.owner : entry.owner,
			classification: current.classification ?? entry.classification,
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
		"# Resource Migration Map",
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
		"- Every resource must have a target, owner, cutover handling and verification checks.",
		"- Source rows, domains and summary counters must match the inventory-derived map.",
		"- Generation provenance must identify source, write command and strict cutover command.",
		"",
		"## Resources",
		"",
		"| Type | Name | Decision | Owner | Target |",
		"|---|---|---:|---|---|",
	];
	for (const entry of data.entries) {
		lines.push(`| ${entry.source.type} | ${entry.source.name} | ${entry.decision} | ${entry.owner} | \`${entry.target}\` |`);
	}
	lines.push(
		"",
		"## Regeneration",
		"",
		"```bash",
		"pnpm check:migration-resource-map",
		"tools/migration/resource-map.mjs --write",
		"```",
		"",
	);
	return lines.join("\n");
}

function validate(map, expectedEntries) {
	const expectedKeys = new Set(expectedEntries.map(keyFor));
	const seen = new Set();
	if (map.schema_version !== 1) errors.push(`${outputPath}: schema_version must be 1`);
	if (map.generation?.command !== "tools/migration/resource-map.mjs --write") errors.push(`${outputPath}: generation.command is invalid`);
	if (map.generation?.source !== inventoryPath) errors.push(`${outputPath}: generation.source is invalid`);
	if (map.generation?.strict_cutover_command !== "tools/migration/resource-map.mjs --strict") errors.push(`${outputPath}: generation.strict_cutover_command is invalid`);
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
		if (!entry.cutover_handling) errors.push(`${key}: cutover_handling is required`);
		if (!Array.isArray(entry.verification) || entry.verification.length === 0) errors.push(`${key}: verification checks are required`);
		if (strict) {
			if (entry.decision === "pending") errors.push(`${key}: pending decision blocks cutover`);
			if (entry.owner.endsWith(" required")) errors.push(`${key}: owner must be assigned`);
			if (entry.target.startsWith("pending:")) errors.push(`${key}: target must be resolved`);
		}
	}
	for (const key of expectedKeys) {
		if (!seen.has(key)) errors.push(`${outputPath}: missing entry ${key}`);
	}
}

const inventory = readJson(inventoryPath);
const generatedEntries = inventory ? sourceEntries(inventory) : [];
const existing = existsSync(outputPath) ? readJson(outputPath) : undefined;
const resourceMap = {
	schema_version: 1,
	generation: {
		command: "tools/migration/resource-map.mjs --write",
		source: inventoryPath,
		strict_cutover_command: "tools/migration/resource-map.mjs --strict",
	},
	summary: { entries: generatedEntries.length, pending: 0, keep: 0, rebuild: 0, remove: 0, replace: 0 },
	entries: mergeExisting(generatedEntries, existing),
};

for (const entry of resourceMap.entries) {
	resourceMap.summary[entry.decision] += 1;
}

if (write) {
	mkdirSync(dirname(outputPath), { recursive: true });
	writeFileSync(outputPath, serialize(resourceMap));
	writeFileSync(markdownPath, serializeMarkdown(resourceMap));
	console.log(`Migration resource map written to ${markdownPath} and ${outputPath}`);
	process.exit(0);
}

validate(resourceMap, generatedEntries);

if (!existsSync(outputPath)) {
	errors.push(`${outputPath}: missing; run tools/migration/resource-map.mjs --write`);
} else if (readFileSync(outputPath, "utf8") !== serialize(resourceMap)) {
	errors.push(`${outputPath}: stale; run tools/migration/resource-map.mjs --write`);
}

if (!existsSync(markdownPath)) {
	errors.push(`${markdownPath}: missing; run tools/migration/resource-map.mjs --write`);
} else if (readFileSync(markdownPath, "utf8") !== serializeMarkdown(resourceMap)) {
	errors.push(`${markdownPath}: stale; run tools/migration/resource-map.mjs --write`);
}

if (errors.length > 0) {
	console.error("Migration resource-map checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(`Migration resource map: ok (${resourceMap.summary.entries} entries, ${resourceMap.summary.pending} pending)`);
