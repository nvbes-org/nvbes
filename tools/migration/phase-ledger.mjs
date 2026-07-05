#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { readPackageScripts, validateProofCommand } from "./execution-backlog.proof.mjs";
import { completedPhaseEvidence } from "./phase-ledger.completed.mjs";

const sourcePath = "docs/blueprint/nvbes-full-restructure-big-bang-zero-debt.plan.md";
const outputPath = "docs/migration/phase-ledger.generated.json";
const markdownPath = "docs/migration/phase-ledger.md";
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

function parsePhases(markdown) {
	const lines = markdown.split("\n");
	const start = lines.findIndex((line) => line.trim() === "## Phases de Reconstruction");
	if (start === -1) return [];
	const phases = [];
	for (const line of lines.slice(start + 1)) {
		if (line.startsWith("## ")) break;
		const match = line.match(/^(\d+)\.\s+(.+?)(?:[:.]|$)/);
		if (!match) continue;
		phases.push({
			phase: `P${match[1].padStart(2, "0")}`,
			title: match[2].trim(),
			owner: "migration lead required",
			status: "pending",
			evidence: [],
			decision: "no-go",
			proof: "pending phase implementation evidence",
		});
	}
	return phases;
}

function applyCompletedPhaseEvidence(phase) {
	const evidence = completedPhaseEvidence[phase.phase];
	return evidence ? { ...phase, ...evidence } : phase;
}

function keyFor(phase) {
	return phase.phase;
}

function mergeExisting(generated, existing) {
	if (!existing?.phases) return generated;
	const byKey = new Map(existing.phases.map((phase) => [keyFor(phase), phase]));
	return generated.map((phase) => {
		const current = byKey.get(keyFor(phase));
		if (!current) return phase;
		return {
			...phase,
			owner: current.owner?.endsWith(" required") ? phase.owner : (current.owner ?? phase.owner),
			status: current.status === "pending" ? phase.status : (current.status ?? phase.status),
			evidence: phase.evidence?.length > 0 || current.evidence?.length === 0
				? phase.evidence
				: (current.evidence ?? phase.evidence),
			decision: current.decision === "no-go" ? phase.decision : (current.decision ?? phase.decision),
			proof: phase.evidence?.length > 0 || current.proof === "pending phase implementation evidence"
				? phase.proof
				: (current.proof ?? phase.proof),
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
		"# Migration Phase Ledger",
		"",
		"## Status",
		"",
		`- phases: ${data.summary.phases}`,
		`- pending: ${data.summary.pending}`,
		`- accepted: ${data.summary.accepted}`,
		`- passed: ${data.summary.passed}`,
		`- failed: ${data.summary.failed}`,
		"",
		"## Phases",
		"",
		"| Phase | Status | Decision | Owner | Evidence | Proof |",
		"|---|---:|---:|---|---|---|",
	];
	for (const phase of data.phases) {
		lines.push(`| ${phase.phase} ${phase.title} | ${phase.status} | ${phase.decision} | ${phase.owner} | ${evidenceCell(phase.evidence)} | \`${phase.proof}\` |`);
	}
	lines.push(
		"",
		"## Rules",
		"",
		"- Phase rows must match the blueprint phase keys and titles.",
		"- Summary counters must match the generated phase rows.",
		"- Every `passed` or `accepted` phase evidence path must still exist in the repository.",
		"- Generated Markdown must expose each phase owner, status, decision, evidence and proof.",
		"- Pending cutover phases must expose the live evidence packet and artifacts required to unlock them.",
		"- Generation provenance must identify source, write command and strict cutover command.",
		"",
		"## Regeneration",
		"",
		"```bash",
		"pnpm check:migration-phase-ledger",
		"tools/migration/phase-ledger.mjs --write",
		"```",
		"",
	);
	return lines.join("\n");
}

function validate(ledger, expected) {
	const expectedByKey = new Map(expected.map((phase) => [keyFor(phase), phase]));
	const expectedKeys = new Set(expectedByKey.keys());
	const seen = new Set();
	if (ledger.schema_version !== 1) errors.push(`${outputPath}: schema_version must be 1`);
	if (!Array.isArray(ledger.phases)) errors.push(`${outputPath}: phases must be an array`);
	validateGeneration(ledger);
	validateSummary(ledger);
	for (const phase of ledger.phases ?? []) {
		const key = keyFor(phase);
		if (seen.has(key)) errors.push(`${outputPath}: duplicate phase ${key}`);
		seen.add(key);
		if (!expectedKeys.has(key)) errors.push(`${outputPath}: stale phase ${key}`);
		const expectedPhase = expectedByKey.get(key);
		if (!phase.title) errors.push(`${key}: title is required`);
		else if (expectedPhase && phase.title !== expectedPhase.title) {
			errors.push(`${key}: title must match blueprint phase`);
		}
		if (!phase.owner) errors.push(`${key}: owner is required`);
		if (!["pending", "passed", "accepted", "failed"].includes(phase.status)) errors.push(`${key}: unsupported status ${phase.status}`);
		if (!["go", "no-go"].includes(phase.decision)) errors.push(`${key}: unsupported decision ${phase.decision}`);
		if (!Array.isArray(phase.evidence)) errors.push(`${key}: evidence must be an array`);
		if (!phase.proof) errors.push(`${key}: proof is required`);
		else if (phase.status !== "pending" || phase.decision === "go") {
			errors.push(...validateProofCommand({ id: key, proof: phase.proof }, packageScripts));
		}
		if (["passed", "accepted"].includes(phase.status)) {
			for (const evidencePath of phase.evidence ?? []) {
				if (!existsSync(evidencePath)) errors.push(`${key}: evidence file is missing: ${evidencePath}`);
			}
		}
		if (strict) {
			if (phase.owner.endsWith(" required")) errors.push(`${key}: owner must be assigned`);
			if (phase.status !== "passed" && phase.status !== "accepted") errors.push(`${key}: phase must be passed or accepted`);
			if (phase.decision !== "go") errors.push(`${key}: decision must be go`);
			if (phase.evidence.length === 0) errors.push(`${key}: evidence must be attached`);
		}
	}
	for (const key of expectedKeys) {
		if (!seen.has(key)) errors.push(`${outputPath}: missing phase ${key}`);
	}
}

function validateGeneration(ledger) {
	const generation = ledger.generation ?? {};
	if (generation.command !== "tools/migration/phase-ledger.mjs --write") {
		errors.push(`${outputPath}: generation.command is invalid`);
	}
	if (generation.source !== sourcePath) {
		errors.push(`${outputPath}: generation.source must match blueprint source`);
	}
	if (generation.strict_cutover_command !== "tools/migration/phase-ledger.mjs --strict") {
		errors.push(`${outputPath}: generation.strict_cutover_command is invalid`);
	}
}

function validateSummary(ledger) {
	const phases = Array.isArray(ledger.phases) ? ledger.phases : [];
	const summary = ledger.summary ?? {};
	const counts = { pending: 0, accepted: 0, passed: 0, failed: 0 };
	for (const phase of phases) {
		if (phase.status in counts) counts[phase.status] += 1;
	}
	if (summary.phases !== phases.length) errors.push(`${outputPath}: summary phases must match phase rows`);
	for (const [status, count] of Object.entries(counts)) {
		if (summary[status] !== count) errors.push(`${outputPath}: summary ${status} must match phase rows`);
	}
}

const generated = parsePhases(read(sourcePath)).map(applyCompletedPhaseEvidence);
const existing = readJson(outputPath);
const ledger = {
	schema_version: 1,
	generation: {
		command: "tools/migration/phase-ledger.mjs --write",
		source: sourcePath,
		strict_cutover_command: "tools/migration/phase-ledger.mjs --strict",
	},
	summary: { phases: generated.length, pending: 0, accepted: 0, passed: 0, failed: 0 },
	phases: mergeExisting(generated, existing),
};

for (const phase of ledger.phases) ledger.summary[phase.status] += 1;

const json = serializeJson(ledger);
const markdown = serializeMarkdown(ledger);

if (write) {
	mkdirSync(dirname(outputPath), { recursive: true });
	writeFileSync(outputPath, json);
	writeFileSync(markdownPath, markdown);
	console.log(`Migration phase ledger written to ${markdownPath} and ${outputPath}`);
	process.exit(0);
}

validate(ledger, generated);

for (const [path, expected] of [[outputPath, json], [markdownPath, markdown]]) {
	if (!existsSync(path)) errors.push(`${path}: missing; run tools/migration/phase-ledger.mjs --write`);
	else if (readFileSync(path, "utf8") !== expected) errors.push(`${path}: stale; run tools/migration/phase-ledger.mjs --write`);
}

if (errors.length > 0) {
	console.error("Migration phase-ledger checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(`Migration phase ledger: ok (${ledger.summary.phases} phases, ${ledger.summary.pending} pending)`);
