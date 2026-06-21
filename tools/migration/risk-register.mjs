#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { riskEvidence } from "./risk-register.evidence.mjs";
import { serializeRiskMarkdown } from "./risk-register.markdown.mjs";

const sourcePath = "docs/blueprint/nvbes-full-restructure-big-bang-zero-debt.plan.md";
const outputPath = "docs/migration/risk-register.generated.json";
const markdownPath = "docs/migration/risk-register.md";
const args = process.argv.slice(2);
const write = args.includes("--write");
const strict = args.includes("--strict");
const errors = [];

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

function slug(value) {
	return value
		.normalize("NFD")
		.replace(/[\u0300-\u036f]/g, "")
		.toLowerCase()
		.replace(/[^a-z0-9]+/g, "-")
		.replace(/^-|-$/g, "");
}

function inferOwner(risk) {
	if (risk.includes("data") || risk.includes("storage") || risk.includes("billing")) return "data lead required";
	if (risk.includes("rollback") || risk.includes("event replay")) return "infra lead required";
	if (risk.includes("Cloud/Internal") || risk.includes("OSS")) return "security lead required";
	if (risk.includes("sessions")) return "product lead required";
	return "migration lead required";
}

function applyMitigationEvidence(entry) {
	const mitigation = riskEvidence[entry.id];
	if (!mitigation) return entry;
	return {
		...entry,
		...mitigation,
	};
}

function parseRisks(markdown) {
	const lines = markdown.split("\n");
	const start = lines.findIndex((line) => line.trim() === "## Risques Bloquants");
	if (start === -1) return [];
	const entries = [];
	for (const line of lines.slice(start + 1)) {
		if (line.startsWith("## ")) break;
		if (!line.startsWith("|") || line.includes("---") || line.includes("Risque")) continue;
		const cells = line.split("|").slice(1, -1).map((cell) => cell.trim());
		if (cells.length !== 2) continue;
		const [risk, mitigation] = cells;
		entries.push({
			id: slug(risk),
			risk,
			mitigation,
			owner: inferOwner(risk),
			severity: "blocking",
			status: "pending",
			evidence: "pending",
			cutover_impact: "no-go until mitigated, accepted by owner, or removed from scope",
		});
	}
	return entries.map(applyMitigationEvidence);
}

function mergeExisting(generated, existing) {
	if (!existing?.risks) return generated;
	const byId = new Map(existing.risks.map((risk) => [risk.id, risk]));
	return generated.map((entry) => {
		const current = byId.get(entry.id);
		if (!current) return entry;
		return {
			...entry,
			owner: current.owner?.endsWith(" required") ? entry.owner : (current.owner ?? entry.owner),
			severity: current.severity ?? entry.severity,
			status: current.status === "pending" ? entry.status : (current.status ?? entry.status),
			evidence: current.evidence === "pending" ? entry.evidence : (current.evidence ?? entry.evidence),
			cutover_impact:
				current.cutover_impact === "no-go until mitigated, accepted by owner, or removed from scope"
					? entry.cutover_impact
					: (current.cutover_impact ?? entry.cutover_impact),
		};
	});
}

function serialize(data) {
	const lines = [
		"{",
		`  "schema_version": ${JSON.stringify(data.schema_version)},`,
		`  "generation": ${JSON.stringify(data.generation)},`,
		`  "summary": ${JSON.stringify(data.summary)},`,
		'  "risks": [',
	];
	for (const [index, risk] of data.risks.entries()) {
		const suffix = index === data.risks.length - 1 ? "" : ",";
		lines.push(`    ${JSON.stringify(risk)}${suffix}`);
	}
	lines.push("  ]", "}");
	return `${lines.join("\n")}\n`;
}

function validate(register, expectedRisks) {
	const expectedById = new Map(expectedRisks.map((risk) => [risk.id, risk]));
	const expectedIds = new Set(expectedById.keys());
	const seen = new Set();
	if (register.schema_version !== 1) errors.push(`${outputPath}: schema_version must be 1`);
	if (!Array.isArray(register.risks)) errors.push(`${outputPath}: risks must be an array`);
	validateGeneration(register);
	validateSummary(register);
	for (const risk of register.risks ?? []) {
		if (seen.has(risk.id)) errors.push(`${outputPath}: duplicate risk ${risk.id}`);
		seen.add(risk.id);
		if (!expectedIds.has(risk.id)) errors.push(`${outputPath}: stale risk ${risk.id}`);
		const expectedRisk = expectedById.get(risk.id);
		if (!risk.risk) errors.push(`${risk.id}: risk is required`);
		else if (expectedRisk && risk.risk !== expectedRisk.risk) {
			errors.push(`${risk.id}: risk text must match blueprint`);
		}
		if (!risk.mitigation) errors.push(`${risk.id}: mitigation is required`);
		else if (expectedRisk && risk.mitigation !== expectedRisk.mitigation) {
			errors.push(`${risk.id}: mitigation must match blueprint`);
		}
		if (!risk.owner) errors.push(`${risk.id}: owner is required`);
		if (!["blocking", "high", "medium", "low"].includes(risk.severity)) errors.push(`${risk.id}: unsupported severity ${risk.severity}`);
		if (!["pending", "mitigated", "accepted", "removed"].includes(risk.status)) errors.push(`${risk.id}: unsupported status ${risk.status}`);
		if (!risk.evidence) errors.push(`${risk.id}: evidence is required`);
		if (!risk.cutover_impact) errors.push(`${risk.id}: cutover_impact is required`);
		for (const evidencePath of evidenceFilePaths(risk.evidence)) {
			if (!existsSync(evidencePath)) errors.push(`${risk.id}: evidence file is missing: ${evidencePath}`);
		}
		if (strict) {
			if (risk.status === "pending") errors.push(`${risk.id}: pending risk blocks cutover`);
			if (risk.owner.endsWith(" required")) errors.push(`${risk.id}: owner must be assigned`);
			if (risk.evidence === "pending") errors.push(`${risk.id}: evidence must be attached`);
		}
	}
	for (const id of expectedIds) {
		if (!seen.has(id)) errors.push(`${outputPath}: missing risk ${id}`);
	}
}

function validateGeneration(register) {
	const generation = register.generation ?? {};
	if (generation.command !== "tools/migration/risk-register.mjs --write") {
		errors.push(`${outputPath}: generation.command is invalid`);
	}
	if (generation.source !== sourcePath) {
		errors.push(`${outputPath}: generation.source must match blueprint source`);
	}
	if (generation.strict_cutover_command !== "tools/migration/risk-register.mjs --strict") {
		errors.push(`${outputPath}: generation.strict_cutover_command is invalid`);
	}
}

function validateSummary(register) {
	const risks = Array.isArray(register.risks) ? register.risks : [];
	const summary = register.summary ?? {};
	const counts = { pending: 0, mitigated: 0, accepted: 0, removed: 0 };
	for (const risk of risks) {
		if (risk.status in counts) counts[risk.status] += 1;
	}
	if (summary.risks !== risks.length) errors.push(`${outputPath}: summary risks must match risk rows`);
	for (const [status, count] of Object.entries(counts)) {
		if (summary[status] !== count) errors.push(`${outputPath}: summary ${status} must match risk rows`);
	}
}

function evidenceFilePaths(evidence) {
	if (typeof evidence !== "string" || evidence === "pending") return [];
	return evidence
		.split(";")
		.map((entry) => entry.trim())
		.filter((entry) => /^(apps|contracts|deploy|docs|infrastructure|libs|scripts|tools)\//.test(entry));
}

const generatedRisks = parseRisks(read(sourcePath));
const existing = readJson(outputPath);
const register = {
	schema_version: 1,
	generation: {
		command: "tools/migration/risk-register.mjs --write",
		source: sourcePath,
		strict_cutover_command: "tools/migration/risk-register.mjs --strict",
	},
	summary: { risks: generatedRisks.length, pending: 0, mitigated: 0, accepted: 0, removed: 0 },
	risks: mergeExisting(generatedRisks, existing),
};

for (const risk of register.risks) {
	register.summary[risk.status] += 1;
}

if (write) {
	mkdirSync(dirname(outputPath), { recursive: true });
	writeFileSync(outputPath, serialize(register));
	writeFileSync(markdownPath, serializeRiskMarkdown(register));
	console.log(`Migration risk register written to ${outputPath} and ${markdownPath}`);
	process.exit(0);
}

validate(register, generatedRisks);

if (!existsSync(outputPath)) {
	errors.push(`${outputPath}: missing; run tools/migration/risk-register.mjs --write`);
} else if (readFileSync(outputPath, "utf8") !== serialize(register)) {
	errors.push(`${outputPath}: stale; run tools/migration/risk-register.mjs --write`);
}
if (!existsSync(markdownPath)) {
	errors.push(`${markdownPath}: missing; run tools/migration/risk-register.mjs --write`);
} else if (readFileSync(markdownPath, "utf8") !== serializeRiskMarkdown(register)) {
	errors.push(`${markdownPath}: stale; run tools/migration/risk-register.mjs --write`);
}

if (errors.length > 0) {
	console.error("Migration risk-register checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(`Migration risk register: ok (${register.summary.risks} risks, ${register.summary.pending} pending)`);
