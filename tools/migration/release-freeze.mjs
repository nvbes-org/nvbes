#!/usr/bin/env node
import { readFileSync } from "node:fs";

const args = process.argv.slice(2);
const strict = args.includes("--strict");
const path = "docs/migration/release-freeze-manifest.md";
const content = readFileSync(path, "utf8");

const requiredHeadings = [
	"# Release Freeze Manifest",
	"## Status",
	"## Rules",
	"## Release Inputs",
	"## Operational Freezes",
	"## Decision",
];

const requiredInputs = [
	"target commit",
	"Rust API images",
	"worker images",
	"frontend artifacts",
	"SQL migrations",
	"OpenAPI contracts",
	"Protobuf contracts",
	"event schemas",
];

const requiredFreezes = [
	"capacity verified",
	"production secrets present",
	"deploy freeze active",
	"billing mutation freeze",
	"rollback target confirmed",
];

const releaseColumns = ["Input", "Owner", "Evidence", "Status", "Decision"];
const freezeColumns = ["Freeze", "Owner", "Evidence", "Status", "Decision"];
const validStatuses = ["pending", "passed", "accepted", "failed", "blocking"];
const validDecisions = ["go", "no-go"];
const errors = [];

for (const heading of requiredHeadings) {
	if (!content.includes(heading)) errors.push(`missing heading: ${heading}`);
}

for (const input of requiredInputs) {
	if (!content.includes(`| ${input} |`)) errors.push(`missing release input: ${input}`);
}

for (const freeze of requiredFreezes) {
	if (!content.includes(`| ${freeze} |`)) errors.push(`missing operational freeze: ${freeze}`);
}

validateTable("## Release Inputs", releaseColumns, requiredInputs);
validateTable("## Operational Freezes", freezeColumns, requiredFreezes);
validateStatusSummary();

const releaseInputRows = tableRowsAfter("## Release Inputs");
const operationalFreezeRows = tableRowsAfter("## Operational Freezes");

for (const row of releaseInputRows) {
	validateFreezeRow(row);
}
for (const row of operationalFreezeRows) {
	validateFreezeRow(row);
	validateOperationalFreezeDependency(row);
}

if (!content.includes("Cutover remains no-go until every release input and operational freeze row")) {
	errors.push("missing explicit no-go cutover decision");
}

if (strict) {
	const blockers = strictBlockers();
	for (const [blocker, count] of Object.entries(blockers)) {
		if (count > 0) errors.push(`${count} ${blocker} release-freeze marker(s) remain`);
	}
}

function tableRowsAfter(heading) {
	const start = content.indexOf(heading);
	if (start === -1) return [];
	const section = content.slice(start).split(/\n## /)[0];
	return section
		.split("\n")
		.filter((line) => line.startsWith("|") && !line.includes("---") && !line.includes("Input |") && !line.includes("Freeze |"))
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
	const labels = rows.map((row) => row[0]);
	if (rows.length !== expectedLabels.length) {
		errors.push(`${heading}: expected ${expectedLabels.length} rows, found ${rows.length}`);
	}
	for (const label of expectedLabels) {
		if (!labels.includes(label)) errors.push(`${heading}: missing row ${label}`);
	}
	for (const label of labels) {
		if (!expectedLabels.includes(label)) errors.push(`${heading}: unexpected row ${label}`);
	}
}

function validateFreezeRow(row) {
	const [label, owner, evidence, status, decision] = row;
	if (row.length !== 5) errors.push(`${label}: expected 5 columns`);
	if (["pending", "none", ""].includes(owner)) errors.push(`${label}: owner is required`);
	if (!validStatuses.includes(status)) {
		errors.push(`${label}: unsupported release-freeze status ${status}`);
	}
	if (!validDecisions.includes(decision)) errors.push(`${label}: unsupported decision ${decision}`);
	if (decision === "go") {
		if (["pending", "none", ""].includes(evidence)) errors.push(`${label}: go decision requires evidence`);
		if (!hasConcreteEvidence(evidence)) {
			errors.push(`${label}: go decision requires immutable concrete evidence`);
		}
		if (!["passed", "accepted"].includes(status)) errors.push(`${label}: go decision requires passed or accepted status`);
	}
	if (["pending", "failed", "blocking"].includes(status) && decision !== "no-go") {
		errors.push(`${label}: ${status} status requires no-go decision`);
	}
}

function validateOperationalFreezeDependency(row) {
	const [freeze, , , , decision] = row;
	if (decision !== "go") return;
	for (const [input, , , , inputDecision] of releaseInputRows) {
		if (inputDecision !== "go") {
			errors.push(`${freeze}: go decision requires ${input} release input`);
		}
	}
}

function validateStatusSummary() {
	const rows = [...tableRowsAfter("## Release Inputs"), ...tableRowsAfter("## Operational Freezes")];
	const statusCounts = countValues(rows.map((row) => row.at(-2)), validStatuses);
	const decisionCounts = countValues(rows.map((row) => row.at(-1)), validDecisions);
	const expected = {
		total_decision_rows: rows.length,
		pending_rows: statusCounts.pending,
		passed_rows: statusCounts.passed,
		accepted_rows: statusCounts.accepted,
		failed_rows: statusCounts.failed,
		blocking_rows: statusCounts.blocking,
		go_decisions: decisionCounts.go,
		no_go_decisions: decisionCounts["no-go"],
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

function hasConcreteEvidence(value) {
	const normalized = (value ?? "").toLowerCase();
	return /(sha|commit|digest|image|artifact|manifest|migration|contract|schema|report|result|record|signed|freeze|capacity|secret|rollback|version|url|link)/.test(normalized);
}

function strictBlockers() {
	const counts = { pending: 0, blocking: 0, failed: 0, "no-go": 0 };
	for (const row of [...tableRowsAfter("## Release Inputs"), ...tableRowsAfter("## Operational Freezes")]) {
		const status = row.at(-2);
		const decision = row.at(-1);
		if (status === "pending") counts.pending += 1;
		if (status === "blocking") counts.blocking += 1;
		if (status === "failed") counts.failed += 1;
		if (decision === "no-go") counts["no-go"] += 1;
	}
	return counts;
}

if (errors.length > 0) {
	console.error("Release freeze checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(
	`Release freeze: ok (${requiredInputs.length} inputs, ${requiredFreezes.length} freezes)`,
);
