#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { gateEvidence } from "./gate-evidence.decisions.mjs";
import { serializeGateEvidenceMarkdown } from "./gate-evidence.markdown.mjs";

const sourcePath = "docs/blueprint/nvbes-full-restructure-big-bang-zero-debt.plan.md";
const outputPath = "docs/migration/gate-evidence.generated.json";
const markdownPath = "docs/migration/gate-evidence.md";
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

function parseGates(markdown) {
	const lines = markdown.split("\n");
	const start = lines.findIndex((line) => line.trim() === "## Gates de Decision");
	if (start === -1) return [];
	const gates = [];
	for (const line of lines.slice(start + 1)) {
		if (line.startsWith("## ")) break;
		if (!line.startsWith("|") || line.includes("---") || line.includes("Gate")) continue;
		const cells = line.split("|").slice(1, -1).map((cell) => cell.trim());
		if (cells.length !== 3) continue;
		const [gate, go_condition, no_go_condition] = cells;
		gates.push({
			gate,
			go_condition,
			no_go_condition,
			owner: "migration lead required",
			status: "pending",
			evidence: [],
			decision: "no-go",
			notes: "pending gate review",
		});
	}
	return gates;
}

function keyFor(entry) {
	return entry.gate.split(" ")[0];
}

function applyGateEvidence(entry) {
	const evidence = gateEvidence[keyFor(entry)];
	return evidence ? { ...entry, ...evidence } : entry;
}

function mergeExisting(generated, existing) {
	if (!existing?.gates) return generated;
	const byKey = new Map(existing.gates.map((gate) => [keyFor(gate), gate]));
	return generated.map((entry) => {
		const current = byKey.get(keyFor(entry));
		if (!current) return entry;
		return {
			...entry,
			owner: current.owner === "migration lead required" ? entry.owner : (current.owner ?? entry.owner),
			status: current.status === "pending" ? entry.status : (current.status ?? entry.status),
			evidence: current.evidence?.length === 0 ? entry.evidence : (current.evidence ?? entry.evidence),
			decision: current.decision === "no-go" ? entry.decision : (current.decision ?? entry.decision),
			notes: current.notes === "pending gate review" ? entry.notes : (current.notes ?? entry.notes),
		};
	});
}

function serialize(data) {
	const lines = [
		"{",
		`  "schema_version": ${JSON.stringify(data.schema_version)},`,
		`  "generation": ${JSON.stringify(data.generation)},`,
		`  "summary": ${JSON.stringify(data.summary)},`,
		'  "gates": [',
	];
	for (const [index, gate] of data.gates.entries()) {
		const suffix = index === data.gates.length - 1 ? "" : ",";
		lines.push(`    ${JSON.stringify(gate)}${suffix}`);
	}
	lines.push("  ]", "}");
	return `${lines.join("\n")}\n`;
}

function validate(register, expectedGates) {
	const expectedByKey = new Map(expectedGates.map((gate) => [keyFor(gate), gate]));
	const expectedKeys = new Set(expectedByKey.keys());
	const seen = new Set();
	if (register.schema_version !== 1) errors.push(`${outputPath}: schema_version must be 1`);
	if (!Array.isArray(register.gates)) errors.push(`${outputPath}: gates must be an array`);
	validateGeneration(register);
	validateSummary(register);
	for (const gate of register.gates ?? []) {
		const key = keyFor(gate);
		if (seen.has(key)) errors.push(`${outputPath}: duplicate gate ${key}`);
		seen.add(key);
		if (!expectedKeys.has(key)) errors.push(`${outputPath}: stale gate ${key}`);
		const expectedGate = expectedByKey.get(key);
		if (expectedGate) {
			if (gate.gate !== expectedGate.gate) errors.push(`${key}: gate label must match blueprint`);
			if (gate.go_condition !== expectedGate.go_condition) {
				errors.push(`${key}: go_condition must match blueprint`);
			}
			if (gate.no_go_condition !== expectedGate.no_go_condition) {
				errors.push(`${key}: no_go_condition must match blueprint`);
			}
		}
		if (!gate.go_condition) errors.push(`${key}: go_condition is required`);
		if (!gate.no_go_condition) errors.push(`${key}: no_go_condition is required`);
		if (!gate.owner) errors.push(`${key}: owner is required`);
		if (!["pending", "passed", "accepted", "failed"].includes(gate.status)) errors.push(`${key}: unsupported status ${gate.status}`);
		if (!["go", "no-go"].includes(gate.decision)) errors.push(`${key}: unsupported decision ${gate.decision}`);
		if (!Array.isArray(gate.evidence)) errors.push(`${key}: evidence must be an array`);
		if (!gate.notes) errors.push(`${key}: notes are required`);
		for (const evidencePath of gate.evidence ?? []) {
			if (!existsSync(evidencePath)) errors.push(`${key}: evidence file is missing: ${evidencePath}`);
		}
		if (strict) {
			if (gate.owner === "migration lead required") errors.push(`${key}: owner must be assigned`);
			if (gate.status !== "passed" && gate.status !== "accepted") errors.push(`${key}: gate must be passed or accepted`);
			if (gate.decision !== "go") errors.push(`${key}: decision must be go`);
			if (gate.evidence.length === 0) errors.push(`${key}: evidence must be attached`);
		}
	}
	for (const key of expectedKeys) {
		if (!seen.has(key)) errors.push(`${outputPath}: missing gate ${key}`);
	}
}

function validateGeneration(register) {
	const generation = register.generation ?? {};
	if (generation.command !== "node tools/migration/gate-evidence.mjs --write") {
		errors.push(`${outputPath}: generation.command is invalid`);
	}
	if (generation.source !== sourcePath) {
		errors.push(`${outputPath}: generation.source must match blueprint source`);
	}
	if (generation.strict_cutover_command !== "node tools/migration/gate-evidence.mjs --strict") {
		errors.push(`${outputPath}: generation.strict_cutover_command is invalid`);
	}
}

function validateSummary(register) {
	const gates = Array.isArray(register.gates) ? register.gates : [];
	const summary = register.summary ?? {};
	const counts = { pending: 0, passed: 0, accepted: 0, failed: 0 };
	for (const gate of gates) {
		if (gate.status in counts) counts[gate.status] += 1;
	}
	if (summary.gates !== gates.length) errors.push(`${outputPath}: summary gates must match gate rows`);
	for (const [status, count] of Object.entries(counts)) {
		if (summary[status] !== count) errors.push(`${outputPath}: summary ${status} must match gate rows`);
	}
}

const generatedGates = parseGates(read(sourcePath)).map(applyGateEvidence);
const existing = readJson(outputPath);
const register = {
	schema_version: 1,
	generation: {
		command: "node tools/migration/gate-evidence.mjs --write",
		source: sourcePath,
		strict_cutover_command: "node tools/migration/gate-evidence.mjs --strict",
	},
	summary: { gates: generatedGates.length, pending: 0, passed: 0, accepted: 0, failed: 0 },
	gates: mergeExisting(generatedGates, existing),
};

for (const gate of register.gates) {
	register.summary[gate.status] += 1;
}

if (write) {
	mkdirSync(dirname(outputPath), { recursive: true });
	writeFileSync(outputPath, serialize(register));
	writeFileSync(markdownPath, serializeGateEvidenceMarkdown(register));
	console.log(`Migration gate evidence written to ${outputPath} and ${markdownPath}`);
	process.exit(0);
}

validate(register, generatedGates);

if (!existsSync(outputPath)) {
	errors.push(`${outputPath}: missing; run node tools/migration/gate-evidence.mjs --write`);
} else if (readFileSync(outputPath, "utf8") !== serialize(register)) {
	errors.push(`${outputPath}: stale; run node tools/migration/gate-evidence.mjs --write`);
}
if (!existsSync(markdownPath)) {
	errors.push(`${markdownPath}: missing; run node tools/migration/gate-evidence.mjs --write`);
} else if (readFileSync(markdownPath, "utf8") !== serializeGateEvidenceMarkdown(register)) {
	errors.push(`${markdownPath}: stale; run node tools/migration/gate-evidence.mjs --write`);
}

if (errors.length > 0) {
	console.error("Migration gate-evidence checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(`Migration gate evidence: ok (${register.summary.gates} gates, ${register.summary.pending} pending)`);
