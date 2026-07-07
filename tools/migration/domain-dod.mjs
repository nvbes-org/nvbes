#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { evidenceForCriterion } from "./domain-dod.evidence.mjs";
import { readPackageScripts, validateProofCommand } from "./execution-backlog.proof.mjs";

const blueprintPath = "docs/blueprint/nvbes-full-restructure-big-bang-zero-debt.plan.md";
const domainsPath = "docs/migration/domain-ledger.generated.json";
const outputPath = "docs/migration/domain-dod.generated.json";
const markdownPath = "docs/migration/domain-dod.md";
const args = process.argv.slice(2);
const write = args.includes("--write");
const strict = args.includes("--strict");
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
	if (!existsSync(path)) return undefined;
	try {
		return JSON.parse(readFileSync(path, "utf8"));
	} catch (error) {
		errors.push(`${path}: invalid JSON: ${error.message}`);
		return undefined;
	}
}

function slugify(value) {
	return value
		.normalize("NFD")
		.replace(/[\u0300-\u036f]/g, "")
		.toLowerCase()
		.replace(/`/g, "")
		.replace(/[^a-z0-9]+/g, "-")
		.replace(/^-|-$/g, "")
		.slice(0, 54);
}

function parseCriteria(markdown) {
	const lines = markdown.split("\n");
	const start = lines.findIndex((line) => line.trim() === "## Definition de Done Domaine");
	if (start === -1) return [];
	const criteria = [];
	for (const line of lines.slice(start + 1)) {
		if (line.startsWith("## ")) break;
		if (!line.startsWith("- ")) continue;
		const criterion = line.slice(2).replace(/;$/, "").replace(/\.$/, "");
		criteria.push({
			criterion_id: slugify(criterion),
			criterion,
		});
	}
	return criteria;
}

function generatedEntries(domains, criteria) {
	return domains.flatMap((domain) =>
		criteria.map((criterion) => ({
			domain: domain.domain,
			...criterion,
			owner: "product lead required",
			status: "pending",
			evidence: [],
			decision: "no-go",
			proof: "pending domain definition-of-done evidence",
		})),
	);
}

function applyDomainEvidence(generated, domains) {
	const byDomain = new Map(domains.map((domain) => [domain.domain, domain]));
	return generated.map((entry) => {
		const domain = byDomain.get(entry.domain);
		return domain ? evidenceForCriterion(entry, domain) : entry;
	});
}

function keyFor(entry) {
	return `${entry.domain}#${entry.criterion_id}`;
}

function mergeExisting(generated, existing) {
	if (!existing?.entries) return generated;
	const byKey = new Map(existing.entries.map((entry) => [keyFor(entry), entry]));
	return generated.map((entry) => {
		const current = byKey.get(keyFor(entry));
		if (!current) return entry;
		return {
			...entry,
			owner: current.owner?.endsWith(" required") ? entry.owner : (current.owner ?? entry.owner),
			status: current.status === "pending" ? entry.status : (current.status ?? entry.status),
			evidence:
				entry.evidence?.length > 0 || current.evidence?.length === 0
					? entry.evidence
					: (current.evidence ?? entry.evidence),
			decision: current.decision === "no-go" ? entry.decision : (current.decision ?? entry.decision),
			proof:
				current.proof === "pending domain definition-of-done evidence"
					? entry.proof
					: (current.proof ?? entry.proof),
		};
	});
}

function serializeJson(data) {
	return `${JSON.stringify(data, null, 2)}\n`;
}

function evidenceCell(entry) {
	return entry.evidence.map((evidencePath) => `\`${evidencePath}\``).join("<br>");
}

function serializeMarkdown(data) {
	const lines = [
		"# Domain Definition of Done Ledger",
		"",
		"## Status",
		"",
		`- domains: ${data.summary.domains}`,
		`- criteria: ${data.summary.criteria}`,
		`- entries: ${data.summary.entries}`,
		`- pending: ${data.summary.pending}`,
		`- accepted: ${data.summary.accepted}`,
		`- passed: ${data.summary.passed}`,
		`- failed: ${data.summary.failed}`,
		"",
		"## Matrix",
		"",
		"| Domain | Criterion | Status | Decision | Owner | Evidence | Proof |",
		"|---|---|---:|---:|---|---|---|",
	];
	for (const entry of data.entries) {
		lines.push(
			`| ${entry.domain} | ${entry.criterion_id} | ${entry.status} | ${entry.decision} | ${entry.owner} | ${evidenceCell(entry)} | \`${entry.proof}\` |`,
		);
	}
	lines.push(
		"",
		"## Rules",
		"",
		"- Criteria must match the blueprint Definition de Done Domaine section.",
		"- Matrix entries must match the generated domain and criterion pairs.",
		"- Summary counters must match generated matrix rows.",
		"- Every `passed` or `accepted` criterion evidence path must still exist in the repository.",
		"- Markdown rows must expose the evidence attached to each criterion.",
		"- Generation provenance must identify sources, write command and strict cutover command.",
		"",
		"## Regeneration",
		"",
		"```bash",
		"pnpm check:migration-domain-dod",
		"node tools/migration/domain-dod.mjs --write",
		"```",
		"",
	);
	return lines.join("\n");
}

function validate(ledger, generated) {
	const expectedByKey = new Map(generated.map((entry) => [keyFor(entry), entry]));
	const expectedKeys = new Set(expectedByKey.keys());
	const seen = new Set();
	if (ledger.schema_version !== 1) errors.push(`${outputPath}: schema_version must be 1`);
	if (ledger.generation?.command !== "node tools/migration/domain-dod.mjs --write") errors.push(`${outputPath}: generation.command is invalid`);
	if (JSON.stringify(ledger.generation?.sources) !== JSON.stringify([blueprintPath, domainsPath])) errors.push(`${outputPath}: generation.sources is invalid`);
	if (ledger.generation?.strict_cutover_command !== "node tools/migration/domain-dod.mjs --strict") errors.push(`${outputPath}: generation.strict_cutover_command is invalid`);
	if (!Array.isArray(ledger.entries)) errors.push(`${outputPath}: entries must be an array`);
	validateCriteria(ledger);
	validateSummary(ledger);
	for (const entry of ledger.entries ?? []) {
		const key = keyFor(entry);
		if (seen.has(key)) errors.push(`${outputPath}: duplicate entry ${key}`);
		seen.add(key);
		if (!expectedKeys.has(key)) errors.push(`${outputPath}: stale entry ${key}`);
		const expectedEntry = expectedByKey.get(key);
		if (!entry.domain) errors.push(`${key}: domain is required`);
		if (!entry.criterion_id) errors.push(`${key}: criterion_id is required`);
		if (!entry.criterion) errors.push(`${key}: criterion is required`);
		else if (expectedEntry && entry.criterion !== expectedEntry.criterion) {
			errors.push(`${key}: criterion must match blueprint criterion`);
		}
		if (!entry.owner) errors.push(`${key}: owner is required`);
		if (!["pending", "passed", "accepted", "failed"].includes(entry.status)) errors.push(`${key}: unsupported status ${entry.status}`);
		if (!["go", "no-go"].includes(entry.decision)) errors.push(`${key}: unsupported decision ${entry.decision}`);
		if (!Array.isArray(entry.evidence)) errors.push(`${key}: evidence must be an array`);
		if (!entry.proof) errors.push(`${key}: proof is required`);
		else if (entry.status !== "pending" || entry.decision === "go") {
			errors.push(...validateProofCommand({ id: key, proof: entry.proof }, packageScripts));
		}
		if (["passed", "accepted"].includes(entry.status)) {
			for (const evidencePath of entry.evidence ?? []) {
				if (!existsSync(evidencePath)) errors.push(`${key}: evidence file is missing: ${evidencePath}`);
			}
		}
		if (strict) {
			if (entry.owner.endsWith(" required")) errors.push(`${key}: owner must be assigned`);
			if (entry.status !== "passed" && entry.status !== "accepted") errors.push(`${key}: criterion must be passed or accepted`);
			if (entry.decision !== "go") errors.push(`${key}: decision must be go`);
			if (entry.evidence.length === 0) errors.push(`${key}: evidence must be attached`);
		}
	}
	for (const key of expectedKeys) {
		if (!seen.has(key)) errors.push(`${outputPath}: missing entry ${key}`);
	}
}

function validateCriteria(ledger) {
	const parsedCriteria = parseCriteria(read(blueprintPath));
	const actual = Array.isArray(ledger.criteria) ? ledger.criteria : [];
	if (actual.length !== parsedCriteria.length) errors.push(`${outputPath}: criteria count must match blueprint`);
	for (const [index, expected] of parsedCriteria.entries()) {
		const current = actual[index];
		if (!current) continue;
		if (current.criterion_id !== expected.criterion_id) {
			errors.push(`${outputPath}: criterion ${index + 1} id must match blueprint`);
		}
		if (current.criterion !== expected.criterion) {
			errors.push(`${outputPath}: criterion ${index + 1} text must match blueprint`);
		}
	}
}

function validateSummary(ledger) {
	const entries = Array.isArray(ledger.entries) ? ledger.entries : [];
	const criteria = Array.isArray(ledger.criteria) ? ledger.criteria : [];
	const domains = new Set(entries.map((entry) => entry.domain));
	const summary = ledger.summary ?? {};
	const counts = { pending: 0, accepted: 0, passed: 0, failed: 0 };
	for (const entry of entries) {
		if (entry.status in counts) counts[entry.status] += 1;
	}
	if (summary.domains !== domains.size) errors.push(`${outputPath}: summary domains must match matrix rows`);
	if (summary.criteria !== criteria.length) errors.push(`${outputPath}: summary criteria must match criteria rows`);
	if (summary.entries !== entries.length) errors.push(`${outputPath}: summary entries must match matrix rows`);
	for (const [status, count] of Object.entries(counts)) {
		if (summary[status] !== count) errors.push(`${outputPath}: summary ${status} must match matrix rows`);
	}
}

const domainLedger = readJson(domainsPath);
const domains = domainLedger?.domains ?? [];
const criteria = parseCriteria(read(blueprintPath));
const generated = applyDomainEvidence(generatedEntries(domains, criteria), domains);
const existing = readJson(outputPath);
const ledger = {
	schema_version: 1,
	generation: {
		command: "node tools/migration/domain-dod.mjs --write",
		sources: [blueprintPath, domainsPath],
		strict_cutover_command: "node tools/migration/domain-dod.mjs --strict",
	},
	summary: { domains: domains.length, criteria: criteria.length, entries: generated.length, pending: 0, accepted: 0, passed: 0, failed: 0 },
	criteria,
	entries: mergeExisting(generated, existing),
};

for (const entry of ledger.entries) ledger.summary[entry.status] += 1;

const json = serializeJson(ledger);
const markdown = serializeMarkdown(ledger);

if (write) {
	mkdirSync(dirname(outputPath), { recursive: true });
	writeFileSync(outputPath, json);
	writeFileSync(markdownPath, markdown);
	console.log(`Migration domain DoD written to ${markdownPath} and ${outputPath}`);
	process.exit(0);
}

validate(ledger, generated);

for (const [path, expected] of [[outputPath, json], [markdownPath, markdown]]) {
	if (!existsSync(path)) errors.push(`${path}: missing; run node tools/migration/domain-dod.mjs --write`);
	else if (readFileSync(path, "utf8") !== expected) errors.push(`${path}: stale; run node tools/migration/domain-dod.mjs --write`);
}

if (errors.length > 0) {
	console.error("Migration domain DoD checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(`Migration domain DoD: ok (${ledger.summary.entries} entries, ${ledger.summary.pending} pending)`);
