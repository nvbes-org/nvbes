#!/usr/bin/env node
import { readFileSync } from "node:fs";

const args = process.argv.slice(2);
const strict = args.includes("--strict");
const path = "docs/migration/rejects.md";
const content = readFileSync(path, "utf8");

const requiredHeadings = [
	"# Migration Rejects",
	"## Status",
	"## Rules",
	"## Reject Log",
];

const requiredColumns = [
	"Run",
	"Domain",
	"Source",
	"Identifier",
	"Class",
	"Owner",
	"Impact",
	"Evidence",
	"Decision",
];

const requiredRules = [
	"Every rejected row or object",
	"`blocking` rejects stop cutover",
	"`accepted` rejects require owner approval",
];

const errors = [];

for (const heading of requiredHeadings) {
	if (!content.includes(heading)) errors.push(`missing heading: ${heading}`);
}

for (const column of requiredColumns) {
	if (!content.includes(column)) errors.push(`missing reject log column: ${column}`);
}

for (const rule of requiredRules) {
	if (!content.includes(rule)) errors.push(`missing reject rule: ${rule}`);
}

validateRejectLogShape();
validateStatusSummary();

for (const row of tableRowsAfter("## Reject Log")) {
	validateRejectRow(row);
}

if (strict) {
	const blockers = strictBlockers();
	for (const [blocker, count] of Object.entries(blockers)) {
		if (count > 0) errors.push(`${count} ${blocker} reject marker(s) remain`);
	}
}

function tableRowsAfter(heading) {
	const start = content.indexOf(heading);
	if (start === -1) return [];
	const section = content.slice(start).split(/\n## /)[0];
	return section
		.split("\n")
		.filter((line) => line.startsWith("|") && !line.includes("---") && !line.includes("Run |"))
		.map((line) => line.split("|").slice(1, -1).map((cell) => cell.trim()));
}

function tableHeaderAfter(heading) {
	const start = content.indexOf(heading);
	if (start === -1) return [];
	const section = content.slice(start).split(/\n## /)[0];
	const header = section.split("\n").find((line) => line.startsWith("|") && !line.includes("---"));
	return header ? header.split("|").slice(1, -1).map((cell) => cell.trim()) : [];
}

function validateRejectLogShape() {
	const header = tableHeaderAfter("## Reject Log");
	if (header.join("|") !== requiredColumns.join("|")) {
		errors.push(`## Reject Log: columns must be ${requiredColumns.join(", ")}`);
	}
	const rows = tableRowsAfter("## Reject Log");
	if (rows.length === 0) errors.push("## Reject Log: must include at least one row");
	for (const row of rows) {
		if (row.length !== requiredColumns.length) {
			errors.push(`## Reject Log: row ${row[3] ?? "unknown"} must have ${requiredColumns.length} columns`);
		}
	}
}

function validateRejectRow(row) {
	const [run, domain, source, identifier, rejectClass, owner, impact, evidence, decision] = row;
	if (!["blocking", "accepted", "fixed"].includes(rejectClass)) {
		errors.push(`${identifier}: unsupported reject class ${rejectClass}`);
	}
	if (!["no-go", "accepted", "fixed"].includes(decision)) {
		errors.push(`${identifier}: unsupported reject decision ${decision}`);
	}
	if (rejectClass === "blocking" && decision !== "no-go") {
		errors.push(`${identifier}: blocking reject requires no-go decision`);
	}
	if (rejectClass === "accepted" && decision !== "accepted") {
		errors.push(`${identifier}: accepted reject requires accepted decision`);
	}
	if (rejectClass === "fixed" && decision !== "fixed") {
		errors.push(`${identifier}: fixed reject requires fixed decision`);
	}
	if (decision === "accepted") {
		for (const [field, value] of [
			["run", run],
			["domain", domain],
			["source", source],
			["identifier", identifier],
			["owner", owner],
			["impact", impact],
			["evidence", evidence],
		]) {
			if (isEmptyMarker(value)) errors.push(`${identifier}: accepted reject requires ${field}`);
		}
		if (!hasConcreteEvidence(evidence)) {
			errors.push(`${identifier}: accepted reject requires concrete immutable evidence`);
		}
	}
	if (decision === "fixed" && isEmptyMarker(evidence)) {
		errors.push(`${identifier}: fixed reject requires evidence`);
	}
	if (decision === "fixed" && !hasConcreteEvidence(evidence)) {
		errors.push(`${identifier}: fixed reject requires concrete correction evidence`);
	}
}

function validateStatusSummary() {
	const rows = tableRowsAfter("## Reject Log");
	const classes = countValues(rows.map((row) => row[4]), ["blocking", "accepted", "fixed"]);
	const decisions = countValues(rows.map((row) => row.at(-1)), ["no-go", "accepted", "fixed"]);
	const expected = {
		total_reject_rows: rows.length,
		blocking_reject_rows: classes.blocking,
		accepted_reject_rows: classes.accepted,
		fixed_reject_rows: classes.fixed,
		no_go_decisions: decisions["no-go"],
		accepted_decisions: decisions.accepted,
		fixed_decisions: decisions.fixed,
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
	return ["", "none", "pending", "no-go"].includes(value ?? "");
}

function hasConcreteEvidence(value) {
	return /(artifact|report|result|log|journal|record|approval|reconciliation|checksum|snapshot|run|diff|patch|link|url|id)/i.test(value ?? "");
}

function strictBlockers() {
	const counts = { blocking: 0, "no-go": 0, none: 0 };
	for (const row of tableRowsAfter("## Reject Log")) {
		const rejectClass = row[4];
		const decision = row.at(-1);
		if (rejectClass === "blocking") counts.blocking += 1;
		if (decision === "no-go") counts["no-go"] += 1;
		for (const value of row) {
			if (value === "none") counts.none += 1;
		}
	}
	return counts;
}

if (errors.length > 0) {
	console.error("Reject checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(`Rejects: ok (${requiredColumns.length} reject log columns)`);
