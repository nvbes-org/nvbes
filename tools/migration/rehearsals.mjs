#!/usr/bin/env node
import { readFileSync } from "node:fs";

const args = process.argv.slice(2);
const strict = args.includes("--strict");
const path = "docs/migration/rehearsal-ledger.md";
const content = readFileSync(path, "utf8");

const requiredHeadings = [
	"# Migration Rehearsal Ledger",
	"## Status",
	"## Rules",
	"## Required Runs",
	"## Run Evidence",
	"## Stability Review",
	"## Decision",
];

const requiredRuns = ["R1", "R2", "R3"];
const requiredEnvironments = ["local", "staging anonymized", "production snapshot dress rehearsal"];
const requiredEvidence = [
	"Export",
	"Transform",
	"Import",
	"Reconciliation",
	"Smoke",
	"Rollback",
	"Owner Sign-Off",
];
const requiredStabilityChecks = [
	"blocking rejects",
	"row count deltas",
	"checksum mismatch",
	"rollback duration",
	"manual actions",
];

const runColumns = ["Run", "Environment", "Source Snapshot", "Target Version", "Required Evidence", "Status", "Decision"];
const evidenceColumns = ["Run", ...requiredEvidence];
const stabilityColumns = ["Check", "Required State", "Current State", "Decision"];
const validRunStatuses = ["pending", "passed", "accepted", "failed", "blocking"];
const errors = [];

for (const heading of requiredHeadings) {
	if (!content.includes(heading)) errors.push(`missing heading: ${heading}`);
}

for (const run of requiredRuns) {
	if (!content.includes(`| ${run} |`)) errors.push(`missing rehearsal run: ${run}`);
}

for (const environment of requiredEnvironments) {
	if (!content.includes(environment)) errors.push(`missing rehearsal environment: ${environment}`);
}

for (const evidence of requiredEvidence) {
	if (!content.includes(evidence)) errors.push(`missing evidence column: ${evidence}`);
}

for (const check of requiredStabilityChecks) {
	if (!content.includes(`| ${check} |`)) errors.push(`missing stability check: ${check}`);
}

validateTable("## Required Runs", runColumns, requiredRuns);
validateTable("## Run Evidence", evidenceColumns, requiredRuns);
validateTable("## Stability Review", stabilityColumns, requiredStabilityChecks);
validateStatusSummary();

const evidenceByRun = new Map(tableRowsAfter("## Run Evidence").map((row) => [row[0], row]));

for (const row of tableRowsAfter("## Required Runs")) {
	validateRequiredRun(row);
}

for (const row of tableRowsAfter("## Run Evidence")) {
	validateRunEvidence(row);
}

for (const row of tableRowsAfter("## Stability Review")) {
	validateStabilityReview(row);
}

if (strict) {
	const blockers = [...content.matchAll(/\b(pending|no-go)\b/gi)].map((match) =>
		match[1].toLowerCase(),
	);
	if (blockers.length > 0) {
		const counts = blockers.reduce((acc, blocker) => {
			acc[blocker] = (acc[blocker] ?? 0) + 1;
			return acc;
		}, {});
		for (const [blocker, count] of Object.entries(counts)) {
			errors.push(`${count} ${blocker} rehearsal marker(s) remain`);
		}
	}
}

if (!content.includes("Current decision: no-go.")) errors.push("missing current rehearsal no-go decision");
if (!content.includes("G6 cannot pass until this ledger references three completed rehearsal reports")) {
	errors.push("missing explicit rehearsal gate blocker");
}

function tableRowsAfter(heading) {
	const start = content.indexOf(heading);
	if (start === -1) return [];
	const section = content.slice(start).split(/\n## /)[0];
	return section
		.split("\n")
		.filter((line) => line.startsWith("|") && !line.includes("---"))
		.filter((line) => !line.includes("Run |") && !line.includes("Check |"))
		.map((line) => line.split("|").slice(1, -1).map((cell) => cell.trim()));
}

function tableHeaderAfter(heading) {
	const start = content.indexOf(heading);
	if (start === -1) return [];
	const section = content.slice(start).split(/\n## /)[0];
	const header = section.split("\n").find((line) => line.startsWith("|") && !line.includes("---"));
	return header ? header.split("|").slice(1, -1).map((cell) => cell.trim()) : [];
}

function validateTable(heading, expectedColumns, expectedLabels) {
	const header = tableHeaderAfter(heading);
	if (header.join("|") !== expectedColumns.join("|")) {
		errors.push(`${heading}: columns must be ${expectedColumns.join(", ")}`);
	}
	const rows = tableRowsAfter(heading);
	if (rows.length !== expectedLabels.length) {
		errors.push(`${heading}: expected ${expectedLabels.length} rows, found ${rows.length}`);
	}
	const labels = rows.map((row) => row[0]);
	for (const label of expectedLabels) {
		if (!labels.includes(label)) errors.push(`${heading}: missing row ${label}`);
	}
	for (const label of labels) {
		if (!expectedLabels.includes(label)) errors.push(`${heading}: unexpected row ${label}`);
	}
	for (const row of rows) {
		if (row.length !== expectedColumns.length) {
			errors.push(`${heading}: row ${row[0] ?? "unknown"} must have ${expectedColumns.length} columns`);
		}
	}
}

function validateRequiredRun(row) {
	const [run, , sourceSnapshot, targetVersion, evidence, status, decision] = row;
	if (!validRunStatuses.includes(status)) {
		errors.push(`${run}: unsupported rehearsal status ${status}`);
	}
	if (decision === "go") {
		if (!["passed", "accepted"].includes(status)) {
			errors.push(`${run}: go decision requires passed or accepted status`);
		}
		for (const [field, value] of [
			["source snapshot", sourceSnapshot],
			["target version", targetVersion],
			["required evidence", evidence],
		]) {
			if (isEmptyMarker(value)) errors.push(`${run}: go decision requires ${field}`);
		}
		if (!hasConcreteEvidence(sourceSnapshot)) errors.push(`${run}: go decision requires concrete source snapshot`);
		if (!hasConcreteEvidence(targetVersion)) errors.push(`${run}: go decision requires concrete target version`);
		validateGoRunEvidence(run, evidence);
	}
	if (["pending", "failed", "blocking"].includes(status) && decision !== "no-go") {
		errors.push(`${run}: ${status} status requires no-go decision`);
	}
}

function validateGoRunEvidence(run, requiredEvidenceList) {
	const row = evidenceByRun.get(run);
	if (!row) {
		errors.push(`${run}: missing run evidence row`);
		return;
	}
	const cells = Object.fromEntries(evidenceColumns.slice(1).map((column, index) => [column.toLowerCase(), row[index + 1]]));
	for (const required of requiredEvidenceList.split(",").map((value) => value.trim().toLowerCase())) {
		const value = cells[required];
		if (isEmptyMarker(value)) errors.push(`${run}: go decision requires ${required} evidence artifact`);
		else if (!hasConcreteEvidence(value)) errors.push(`${run}: go decision requires concrete ${required} evidence artifact`);
	}
}

function validateRunEvidence(row) {
	const [run, ...evidence] = row;
	for (const item of evidence) {
		if (["passed", "accepted"].includes(item)) {
			errors.push(`${run}: passed or accepted run evidence must reference an immutable artifact`);
		}
	}
}

function validateStabilityReview(row) {
	const [check, , currentState, decision] = row;
	if (decision === "go" && isEmptyMarker(currentState)) {
		errors.push(`${check}: go stability decision requires current state evidence`);
	}
	if (decision === "go" && !hasConcreteEvidence(currentState)) {
		errors.push(`${check}: go stability decision requires concrete current state evidence`);
	}
	if (isEmptyMarker(currentState) && decision !== "no-go") {
		errors.push(`${check}: pending stability state requires no-go decision`);
	}
}

function validateStatusSummary() {
	const runs = tableRowsAfter("## Required Runs");
	const evidenceCells = tableRowsAfter("## Run Evidence").flatMap((row) => row.slice(1));
	const stability = tableRowsAfter("## Stability Review");
	const runStatuses = countValues(runs.map((row) => row.at(-2)), validRunStatuses);
	const runDecisions = countValues(runs.map((row) => row.at(-1)), ["go", "no-go"]);
	const evidenceCounts = countValues(evidenceCells, ["pending"]);
	const stabilityDecisions = countValues(stability.map((row) => row.at(-1)), ["go", "no-go"]);
	const expected = {
		total_run_rows: runs.length,
		pending_run_rows: runStatuses.pending,
		passed_run_rows: runStatuses.passed,
		accepted_run_rows: runStatuses.accepted,
		failed_run_rows: runStatuses.failed,
		blocking_run_rows: runStatuses.blocking,
		run_go_decisions: runDecisions.go,
		run_no_go_decisions: runDecisions["no-go"],
		total_evidence_cells: evidenceCells.length,
		pending_evidence_cells: evidenceCounts.pending,
		total_stability_rows: stability.length,
		stability_go_decisions: stabilityDecisions.go,
		stability_no_go_decisions: stabilityDecisions["no-go"],
	};
	for (const [key, value] of Object.entries(expected)) {
		const actual = statusNumber(key);
		if (actual === null) errors.push(`missing status summary: ${key}`);
		else if (actual !== value) errors.push(`${key}: expected ${value}, found ${actual}`);
	}
}

function countValues(values, knownValues) {
	const counts = Object.fromEntries(knownValues.map((value) => [value, 0]));
	for (const value of values) counts[value] = (counts[value] ?? 0) + 1;
	return counts;
}

function statusNumber(key) {
	const match = content.match(new RegExp(`- \`${key}\`: (\\d+)`));
	return match ? Number.parseInt(match[1], 10) : null;
}

function isEmptyMarker(value) {
	return ["", "pending", "none", "no-go"].includes(value ?? "");
}

function hasConcreteEvidence(value) {
	return /(sha|checksum|digest|artifact|manifest|report|result|log|snapshot|storage|url|link|command output|record|version|reconciliation|rollback|checklist)/i.test(value ?? "");
}

if (errors.length > 0) {
	console.error("Rehearsal checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(
	`Rehearsals: ok (${requiredRuns.length} runs, ${requiredStabilityChecks.length} stability checks)`,
);
