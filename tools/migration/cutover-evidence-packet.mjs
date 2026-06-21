#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { requirements, strictCommandFor } from "./cutover-evidence-packet.requirements.mjs";
import { validatePacket } from "./cutover-evidence-packet.validation.mjs";
import { readPackageScripts } from "./execution-backlog.proof.mjs";
import { buildPreparationCommands, commandCoverageFor } from "./live-evidence-commands.mjs";
import { requiredEvidenceFor } from "./live-evidence.rules.mjs";

const args = process.argv.slice(2);
const write = args.includes("--write");
const strict = args.includes("--strict");
const jsonPath = "docs/migration/cutover-evidence-packet.generated.json";
const markdownPath = "docs/migration/cutover-evidence-packet.md";
const errors = [];
const scripts = readPackageScripts(errors);
let inputState;

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

function keyFor(label) {
	return label?.split(" ")[0];
}

function commandExists(command) {
	return scripts[command] || existsSync(command);
}

function currentDecision(scope, phaseLedger, gateEvidence) {
	const phaseKey = scope.match(/\bP\d+\b/)?.[0];
	const gateKey = scope.match(/\bG\d+\b/)?.[0];
	const phase = phaseLedger?.phases?.find((entry) => entry.phase === phaseKey);
	const gate = gateEvidence?.gates?.find((entry) => keyFor(entry.gate) === gateKey);
	return {
		phase: phase ? `${phase.status}/${phase.decision}` : "not applicable",
		gate: gate ? `${gate.status}/${gate.decision}` : "not applicable",
	};
}

function buildPacket() {
	const phaseLedger = readJson("docs/migration/phase-ledger.generated.json");
	const gateEvidence = readJson("docs/migration/gate-evidence.generated.json");
	const readiness = readJson("docs/migration/readiness-report.generated.json");
	const completion = readJson("docs/migration/completion-audit.generated.json");
	const backlog = readJson("docs/migration/execution-backlog.generated.json");
	inputState = { completion, backlog };
	const rows = requirements.map((requirement) => {
		const missing_files = requirement.files.filter((path) => !existsSync(path));
		const missing_commands = requirement.commands.filter((command) => !commandExists(command));
		const current = currentDecision(requirement.scope, phaseLedger, gateEvidence);
		const repository_ready = missing_files.length === 0 && missing_commands.length === 0;
		const live_blocking_reasons = liveBlockingReasons(requirement.id, repository_ready, current);
		const strict_command = strictCommandFor(requirement);
		const preparation_commands = buildPreparationCommands([{ ...requirement, expected_evidence: requiredEvidenceFor(requirement.id) }]);
		return {
			...requirement,
			current,
			repository_ready,
			live_blocking_reasons,
			missing_files,
			missing_commands,
			strict_command,
			preparation_commands,
			...commandCoverageFor(strict_command, preparation_commands),
		};
	});
	const blocking = rows.filter(isLiveBlocking);
	const readinessGo = readiness?.status?.production_cutover === "go";
	const completionReady = completion?.status?.objective === "complete" && (completion?.status?.incomplete ?? 1) === 0;
	const backlogClear = (backlog?.status?.open_tasks ?? 1) === 0 && (backlog?.status?.blocking_items ?? 1) === 0;
	return {
		schema_version: 1,
		generation: {
			command: "tools/migration/cutover-evidence-packet.mjs --write",
			strict_command: "tools/migration/cutover-evidence-packet.mjs --strict",
		},
		status: {
			readiness: readiness?.status?.production_cutover ?? "unknown",
			repository_ready: rows.every((row) => row.repository_ready),
			live_blocking_items: blocking.length,
			completion: completionReady ? "go" : "no-go",
			backlog_open_tasks: backlog?.status?.open_tasks ?? null,
			backlog_blocking_items: backlog?.status?.blocking_items ?? null,
			decision: readinessGo && blocking.length === 0 && completionReady && backlogClear ? "go" : "no-go",
		},
		requirements: rows,
	};
}

function serializeJson(packet) {
	return `${JSON.stringify(packet, null, 2)}\n`;
}

function serializeMarkdown(packet) {
	const lines = [
		"# Cutover Evidence Packet",
		"",
		"## Status",
		"",
		`- readiness: ${packet.status.readiness}`,
		`- repository_ready: ${packet.status.repository_ready}`,
		`- live_blocking_items: ${packet.status.live_blocking_items}`,
		`- completion: ${packet.status.completion}`,
		`- backlog_open_tasks: ${packet.status.backlog_open_tasks}`,
		`- backlog_blocking_items: ${packet.status.backlog_blocking_items}`,
		`- decision: ${packet.status.decision}`,
		"",
		"## Rules",
		"",
		"- `repository_ready` must equal every requirement repository readiness.",
		"- `live_blocking_items` must equal requirements blocked by missing repository artifacts, pending phase/gate state or final reconciliation.",
		"- Requirement blocking reasons must match current repository, phase, gate and final reconciliation state.",
		"- `decision: go` requires readiness `go`, repository readiness, completion `go`, zero backlog tasks and zero live blockers.",
		"- `decision: go` requires every strict command segment to be covered by a live evidence preparation command.",
		"- Requirement rows must match the static cutover packet contract exactly.",
		"- Generation provenance must identify write and strict check commands.",
		"",
		"## Evidence Requirements",
		"",
		"| ID | Scope | Repository Ready | Current Phase | Current Gate | Blocking Reasons | Uncovered Strict Segments | Live Evidence |",
		"|---|---|---:|---|---|---|---|---|",
	];
	for (const row of packet.requirements) {
		lines.push(
			`| ${row.id} | ${row.scope} | ${row.repository_ready} | ${row.current.phase} | ${row.current.gate} | ${row.live_blocking_reasons.join(", ") || "none"} | ${row.uncovered_strict_command_segments.map((segment) => `\`${segment}\``).join("<br>") || "none"} | ${row.live_evidence} |`,
		);
	}
	lines.push("", "## Strict Commands", "");
	for (const row of packet.requirements) {
		lines.push(`- ${row.id}: \`${row.strict_command}\``);
	}
	lines.push("", "## Preparation Commands", "");
	for (const row of packet.requirements) {
		lines.push(`### ${row.id}`, "", "```bash", ...row.preparation_commands, "```", "");
	}
	lines.push(
		"",
		"## Decision",
		"",
		packet.status.decision === "go"
			? "Cutover evidence is complete."
			: "No-go remains until live rehearsal, cutover, reconciliation and decommission evidence replaces the pending templates.",
		"",
	);
	return lines.join("\n");
}

function validate(packet) {
	errors.push(...validatePacket(packet, { strict, jsonPath, requirements, scripts, liveBlockingReasons, isLiveBlocking, ...inputState }));
}

function isLiveBlocking(row) {
	return (row.live_blocking_reasons ?? liveBlockingReasons(row.id, row.repository_ready, row.current)).length > 0;
}

function liveBlockingReasons(id, repositoryReady, current) {
	const reasons = [];
	if (!repositoryReady) reasons.push("repository artifacts missing");
	if (current?.phase?.includes("pending/no-go")) reasons.push("phase pending/no-go");
	if (current?.gate?.includes("pending/no-go")) reasons.push("gate pending/no-go");
	if (id === "final-reconciliation") reasons.push("production reconciliation not attached");
	return reasons;
}

const packet = buildPacket();
const json = serializeJson(packet);
const markdown = serializeMarkdown(packet);

if (write) {
	mkdirSync(dirname(jsonPath), { recursive: true });
	writeFileSync(jsonPath, json);
	writeFileSync(markdownPath, markdown);
	console.log(`Cutover evidence packet written to ${markdownPath} and ${jsonPath}`);
	process.exit(0);
}

validate(packet);

for (const [path, expected] of [[jsonPath, json], [markdownPath, markdown]]) {
	if (!existsSync(path)) {
		errors.push(`${path}: missing; run tools/migration/cutover-evidence-packet.mjs --write`);
	} else if (readFileSync(path, "utf8") !== expected) {
		errors.push(`${path}: stale; run tools/migration/cutover-evidence-packet.mjs --write`);
	}
}

if (errors.length > 0) {
	console.error("Cutover evidence packet checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(
	`Cutover evidence packet: ok (${packet.status.decision}, ${packet.status.live_blocking_items} live blocking items)`,
);
