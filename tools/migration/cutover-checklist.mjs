#!/usr/bin/env node
import { readFileSync } from "node:fs";

const args = process.argv.slice(2);
const strict = args.includes("--strict");
const path = "docs/migration/cutover-checklist.md";
const content = readFileSync(path, "utf8");

const requiredHeadings = [
	"# Cutover Checklist",
	"## Status",
	"## Rules",
	"## Pre-Cutover",
	"## Cutover Window",
	"## Smoke Tests",
];

const requiredPreCutover = [
	"commit target frozen",
	"images OCI immutable",
	"migrations SQL frozen",
	"latest rehearsal accepted",
	"rollback rehearsal accepted",
	"maintenance announced",
	"secret inventory complete",
	"dashboards open",
	"incident channel open",
];

const requiredWindow = [
	"T-0",
	"T+15m",
	"T+30m",
	"T+60m",
	"T+90m",
	"T+105m",
	"T+120m",
	"T+135m",
	"T+180m",
];

const requiredSmokeTests = [
	"login",
	"MFA",
	"workspace access",
	"upload/download",
	"share link",
	"billing entitlement",
	"webhook replay",
	"privacy export",
	"audit event",
	"worker queue",
];

const preCutoverColumns = ["Item", "Owner", "Evidence", "Status"];
const windowColumns = ["Time", "Action", "Owner", "Evidence", "Status"];
const validStatuses = ["pending", "passed", "accepted", "failed", "blocking"];
const errors = [];

for (const heading of requiredHeadings) {
	if (!content.includes(heading)) errors.push(`missing heading: ${heading}`);
}

for (const item of requiredPreCutover) {
	if (!content.includes(`| ${item} |`)) errors.push(`missing pre-cutover item: ${item}`);
}

for (const time of requiredWindow) {
	if (!content.includes(`| ${time} |`)) errors.push(`missing cutover window step: ${time}`);
}

for (const smokeTest of requiredSmokeTests) {
	if (!content.includes(`- ${smokeTest};`) && !content.includes(`- ${smokeTest}.`)) {
		errors.push(`missing smoke test: ${smokeTest}`);
	}
}

validateTable("## Pre-Cutover", preCutoverColumns, requiredPreCutover);
validateTable("## Cutover Window", windowColumns, requiredWindow);
validateSmokeTests();
validateStatusSummary();

const preCutoverRows = tableRowsAfter("## Pre-Cutover");
const windowRows = tableRowsAfter("## Cutover Window");
const checklistRows = [...preCutoverRows, ...windowRows];

for (const row of checklistRows) {
	validateChecklistRow(row);
}
validateFinalDecisionDependency(preCutoverRows, windowRows);

if (!content.includes("Initialized. Not approved for production.")) {
	errors.push("missing explicit not-approved status");
}

if (strict) {
	const blockers = strictBlockers(checklistRows);
	for (const [blocker, count] of Object.entries(blockers)) {
		if (count > 0) errors.push(`${count} ${blocker} cutover checklist marker(s) remain`);
	}
}

function tableRowsAfter(heading) {
	const start = content.indexOf(heading);
	if (start === -1) return [];
	const section = content.slice(start).split(/\n## /)[0];
	return section
		.split("\n")
		.filter((line) => line.startsWith("|") && !line.includes("---") && !line.includes("Item |") && !line.includes("Time |"))
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

function validateSmokeTests() {
	const sectionStart = content.indexOf("## Smoke Tests");
	if (sectionStart === -1) return;
	const section = content.slice(sectionStart).split(/\n## /)[0];
	const tests = section
		.split("\n")
		.filter((line) => line.startsWith("- "))
		.map((line) => line.replace(/^- /, "").replace(/[.;]$/, ""));
	if (tests.length !== requiredSmokeTests.length) {
		errors.push(`smoke tests: expected ${requiredSmokeTests.length} rows, found ${tests.length}`);
	}
	for (const test of tests) {
		if (!requiredSmokeTests.includes(test)) errors.push(`unexpected smoke test: ${test}`);
	}
}

function validateStatusSummary() {
	const rows = [...tableRowsAfter("## Pre-Cutover"), ...tableRowsAfter("## Cutover Window")];
	const counts = Object.fromEntries(validStatuses.map((status) => [status, 0]));
	for (const row of rows) counts[row.at(-1)] = (counts[row.at(-1)] ?? 0) + 1;
	const expected = {
		total_checklist_rows: rows.length,
		pending_rows: counts.pending,
		passed_rows: counts.passed,
		accepted_rows: counts.accepted,
		failed_rows: counts.failed,
		blocking_rows: counts.blocking,
	};
	for (const [key, value] of Object.entries(expected)) {
		const actual = statusNumber(key);
		if (actual === null) errors.push(`missing status summary: ${key}`);
		else if (actual !== value) errors.push(`${key}: expected ${value}, found ${actual}`);
	}
}

function statusNumber(key) {
	const match = content.match(new RegExp(`- \`${key}\`: (\\d+)`));
	return match ? Number.parseInt(match[1], 10) : null;
}

function validateChecklistRow(row) {
	const label = row[0];
	const owner = row.at(-3);
	const evidence = row.at(-2);
	const status = row.at(-1);
	if (!validStatuses.includes(status)) {
		errors.push(`${label}: unsupported checklist status ${status}`);
	}
	if (["", "none"].includes(owner)) errors.push(`${label}: owner is required`);
	if (["", "none"].includes(evidence)) errors.push(`${label}: evidence is required`);
	if (["passed", "accepted"].includes(status)) {
		if (["pending", "none", ""].includes(owner)) errors.push(`${label}: ${status} status requires owner`);
		if (["pending", "none", ""].includes(evidence)) errors.push(`${label}: ${status} status requires evidence`);
		if (!hasConcreteEvidence(evidence)) {
			errors.push(`${label}: ${status} status requires concrete evidence`);
		}
	}
	if (["failed", "blocking"].includes(status) && ["pending", "none", ""].includes(owner)) {
		errors.push(`${label}: ${status} status requires owner`);
	}
}

function validateFinalDecisionDependency(preCutoverRows, windowRows) {
	const finalRow = windowRows.find((row) => row[0] === "T+180m");
	if (!finalRow || !["passed", "accepted"].includes(finalRow.at(-1))) return;
	const priorRows = [...preCutoverRows, ...windowRows.filter((row) => row[0] !== "T+180m")];
	for (const row of priorRows) {
		if (!["passed", "accepted"].includes(row.at(-1))) {
			errors.push(`T+180m: final decision requires ${row[0]} checklist row`);
		}
	}
}

function strictBlockers(rows) {
	const counts = { pending: 0, blocking: 0, failed: 0 };
	for (const row of rows) {
		const status = row.at(-1);
		if (status === "pending") counts.pending += 1;
		if (status === "blocking") counts.blocking += 1;
		if (status === "failed") counts.failed += 1;
	}
	return counts;
}

function hasConcreteEvidence(value) {
	return /(sha|digest|list|report|link|url|id|snapshot|export|import|reconciliation|edge change|health|decision|flag|channel|dashboard|announcement)/i.test(value ?? "");
}

if (errors.length > 0) {
	console.error("Cutover checklist checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(
	`Cutover checklist: ok (${requiredPreCutover.length} pre-cutover, ${requiredWindow.length} window steps)`,
);
