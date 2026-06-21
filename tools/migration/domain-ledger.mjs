#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { completedDomainEvidence } from "./domain-ledger.evidence.mjs";
import { readPackageScripts, validateProofCommand } from "./execution-backlog.proof.mjs";

const sourcePath = "docs/blueprint/nvbes-full-restructure-big-bang-zero-debt.plan.md";
const outputPath = "docs/migration/domain-ledger.generated.json";
const markdownPath = "docs/migration/domain-ledger.md";
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

function parseDomains(markdown) {
	const lines = markdown.split("\n");
	const start = lines.findIndex((line) => line.trim() === "Domaines de base:");
	if (start === -1) return [];
	const domains = [];
	let current;
	for (const line of lines.slice(start + 1)) {
		if (line.startsWith("Regles:")) break;
		if (line.startsWith("- ")) {
			if (current) domains.push(current);
			const [, name, scope] = line.match(/^- ([^:]+):\s*(.*)$/) ?? [];
			current = name ? { name, scope } : undefined;
			continue;
		}
		if (current && line.trim()) current.scope = `${current.scope} ${line.trim()}`;
	}
	if (current) domains.push(current);
	return domains.map((domain) => ({
		domain: domain.name,
		scope: domain.scope.replace(/\.$/, ""),
		owner: "product lead required",
		status: "pending",
		implementation_evidence: [],
		migration_evidence: [],
		decision: "no-go",
		proof: "pending domain implementation and migration evidence",
	}));
}

function keyFor(domain) {
	return domain.domain;
}

function applyCompletedDomainEvidence(domain) {
	const evidence = completedDomainEvidence[keyFor(domain)];
	return evidence ? { ...domain, ...evidence } : domain;
}

function mergeExisting(generated, existing) {
	if (!existing?.domains) return generated;
	const byKey = new Map(existing.domains.map((domain) => [keyFor(domain), domain]));
	return generated.map((domain) => {
		const current = byKey.get(keyFor(domain));
		if (!current) return domain;
		return {
			...domain,
			owner: current.owner?.endsWith(" required") ? domain.owner : (current.owner ?? domain.owner),
			status: current.status === "pending" ? domain.status : (current.status ?? domain.status),
			implementation_evidence:
				current.implementation_evidence?.length === 0
					? domain.implementation_evidence
					: (current.implementation_evidence ?? domain.implementation_evidence),
			migration_evidence:
				current.migration_evidence?.length === 0
					? domain.migration_evidence
					: (current.migration_evidence ?? domain.migration_evidence),
			decision: current.decision === "no-go" ? domain.decision : (current.decision ?? domain.decision),
			proof:
				current.proof === "pending domain implementation and migration evidence"
					? domain.proof
					: (current.proof ?? domain.proof),
		};
	});
}

function serializeJson(data) {
	return `${JSON.stringify(data, null, 2)}\n`;
}

function evidenceCell(evidence) {
	return evidence.length === 0 ? "none" : evidence.map((item) => `\`${item}\``).join("<br>");
}

function serializeMarkdown(data) {
	const lines = [
		"# Migration Domain Ledger",
		"",
		"## Status",
		"",
		`- domains: ${data.summary.domains}`,
		`- pending: ${data.summary.pending}`,
		`- accepted: ${data.summary.accepted}`,
		`- passed: ${data.summary.passed}`,
		`- failed: ${data.summary.failed}`,
		"",
		"## Domains",
		"",
		"| Domain | Status | Decision | Owner | Implementation Evidence | Migration Evidence | Proof |",
		"|---|---:|---:|---|---|---|---|",
	];
	for (const domain of data.domains) {
		lines.push(`| ${domain.domain} | ${domain.status} | ${domain.decision} | ${domain.owner} | ${evidenceCell(domain.implementation_evidence)} | ${evidenceCell(domain.migration_evidence)} | \`${domain.proof}\` |`);
	}
	lines.push(
		"",
		"## Rules",
		"",
		"- Domain rows must match the blueprint domain names and scopes.",
		"- Summary counters must match generated domain rows.",
		"- Every `passed` or `accepted` domain evidence path must still exist in the repository.",
		"- Generated Markdown must expose implementation evidence, migration evidence and proof for every domain.",
		"- Generation provenance must identify source, write command and strict cutover command.",
		"",
		"## Regeneration",
		"",
		"```bash",
		"pnpm check:migration-domain-ledger",
		"tools/migration/domain-ledger.mjs --write",
		"```",
		"",
	);
	return lines.join("\n");
}

function validate(ledger, expected) {
	const expectedByKey = new Map(expected.map((domain) => [keyFor(domain), domain]));
	const expectedKeys = new Set(expectedByKey.keys());
	const seen = new Set();
	if (ledger.schema_version !== 1) errors.push(`${outputPath}: schema_version must be 1`);
	if (!Array.isArray(ledger.domains)) errors.push(`${outputPath}: domains must be an array`);
	validateGeneration(ledger);
	validateSummary(ledger);
	for (const domain of ledger.domains ?? []) {
		const key = keyFor(domain);
		if (seen.has(key)) errors.push(`${outputPath}: duplicate domain ${key}`);
		seen.add(key);
		if (!expectedKeys.has(key)) errors.push(`${outputPath}: stale domain ${key}`);
		const expectedDomain = expectedByKey.get(key);
		if (!domain.scope) errors.push(`${key}: scope is required`);
		else if (expectedDomain && domain.scope !== expectedDomain.scope) {
			errors.push(`${key}: scope must match blueprint domain`);
		}
		if (!domain.owner) errors.push(`${key}: owner is required`);
		if (!["pending", "passed", "accepted", "failed"].includes(domain.status)) errors.push(`${key}: unsupported status ${domain.status}`);
		if (!["go", "no-go"].includes(domain.decision)) errors.push(`${key}: unsupported decision ${domain.decision}`);
		if (!Array.isArray(domain.implementation_evidence)) errors.push(`${key}: implementation_evidence must be an array`);
		if (!Array.isArray(domain.migration_evidence)) errors.push(`${key}: migration_evidence must be an array`);
		if (!domain.proof) errors.push(`${key}: proof is required`);
		else errors.push(...validateProofCommand({ id: key, proof: domain.proof }, packageScripts));
		if (["passed", "accepted"].includes(domain.status)) {
			for (const evidencePath of [...(domain.implementation_evidence ?? []), ...(domain.migration_evidence ?? [])]) {
				if (!existsSync(evidencePath)) errors.push(`${key}: evidence file is missing: ${evidencePath}`);
			}
		}
		if (strict) {
			if (domain.owner.endsWith(" required")) errors.push(`${key}: owner must be assigned`);
			if (domain.status !== "passed" && domain.status !== "accepted") errors.push(`${key}: domain must be passed or accepted`);
			if (domain.decision !== "go") errors.push(`${key}: decision must be go`);
			if (domain.implementation_evidence.length === 0) errors.push(`${key}: implementation evidence must be attached`);
			if (domain.migration_evidence.length === 0) errors.push(`${key}: migration evidence must be attached`);
		}
	}
	for (const key of expectedKeys) {
		if (!seen.has(key)) errors.push(`${outputPath}: missing domain ${key}`);
	}
}

function validateGeneration(ledger) {
	const generation = ledger.generation ?? {};
	if (generation.command !== "tools/migration/domain-ledger.mjs --write") {
		errors.push(`${outputPath}: generation.command is invalid`);
	}
	if (generation.source !== sourcePath) {
		errors.push(`${outputPath}: generation.source must match blueprint source`);
	}
	if (generation.strict_cutover_command !== "tools/migration/domain-ledger.mjs --strict") {
		errors.push(`${outputPath}: generation.strict_cutover_command is invalid`);
	}
}

function validateSummary(ledger) {
	const domains = Array.isArray(ledger.domains) ? ledger.domains : [];
	const summary = ledger.summary ?? {};
	const counts = { pending: 0, accepted: 0, passed: 0, failed: 0 };
	for (const domain of domains) {
		if (domain.status in counts) counts[domain.status] += 1;
	}
	if (summary.domains !== domains.length) errors.push(`${outputPath}: summary domains must match domain rows`);
	for (const [status, count] of Object.entries(counts)) {
		if (summary[status] !== count) errors.push(`${outputPath}: summary ${status} must match domain rows`);
	}
}

const generated = parseDomains(read(sourcePath)).map(applyCompletedDomainEvidence);
const existing = readJson(outputPath);
const ledger = {
	schema_version: 1,
	generation: {
		command: "tools/migration/domain-ledger.mjs --write",
		source: sourcePath,
		strict_cutover_command: "tools/migration/domain-ledger.mjs --strict",
	},
	summary: { domains: generated.length, pending: 0, accepted: 0, passed: 0, failed: 0 },
	domains: mergeExisting(generated, existing),
};

for (const domain of ledger.domains) ledger.summary[domain.status] += 1;

const json = serializeJson(ledger);
const markdown = serializeMarkdown(ledger);

if (write) {
	mkdirSync(dirname(outputPath), { recursive: true });
	writeFileSync(outputPath, json);
	writeFileSync(markdownPath, markdown);
	console.log(`Migration domain ledger written to ${markdownPath} and ${outputPath}`);
	process.exit(0);
}

validate(ledger, generated);

for (const [path, expected] of [[outputPath, json], [markdownPath, markdown]]) {
	if (!existsSync(path)) errors.push(`${path}: missing; run tools/migration/domain-ledger.mjs --write`);
	else if (readFileSync(path, "utf8") !== expected) errors.push(`${path}: stale; run tools/migration/domain-ledger.mjs --write`);
}

if (errors.length > 0) {
	console.error("Migration domain-ledger checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(`Migration domain ledger: ok (${ledger.summary.domains} domains, ${ledger.summary.pending} pending)`);
