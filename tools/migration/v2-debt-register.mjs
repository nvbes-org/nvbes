#!/usr/bin/env node
import { readFileSync } from "node:fs";

const args = process.argv.slice(2);
const strict = args.includes("--strict");
const path = "docs/migration/v2-debt-register.md";
const content = readFileSync(path, "utf8");
const errors = [];

const requiredHeadings = [
	"# V2 Debt Register",
	"## Status",
	"## Rules",
	"## Debt Review",
	"## Decision",
];
const columns = ["Item", "Owner", "Scope", "V1 Required", "Evidence", "Status", "Decision"];
const rows = tableRowsAfter("## Debt Review");

for (const heading of requiredHeadings) {
	if (!content.includes(heading)) errors.push(`missing heading: ${heading}`);
}
validateTable();
validateStatusSummary();
for (const row of rows) validateDebtRow(row);

if (!content.includes("The migration remains incomplete until every V1-required debt item is closed or removed from scope by owner acceptance")) {
	errors.push("missing explicit V1 debt blocker");
}
if (strict) {
	for (const row of rows) {
		const [item, , , v1Required, , status, decision] = row;
		if (isYes(v1Required) && (status !== "closed" || decision !== "go")) {
			errors.push(`${item}: V1-required debt must be closed with go decision`);
		}
	}
}

function tableRowsAfter(heading) {
	const start = content.indexOf(heading);
	if (start === -1) return [];
	const section = content.slice(start).split(/\n## /)[0];
	return section
		.split("\n")
		.filter((line) => line.startsWith("|") && !line.includes("---") && !line.includes("Item |"))
		.map((line) => line.split("|").slice(1, -1).map((cell) => cell.trim()));
}

function tableHeaderAfter(heading) {
	const start = content.indexOf(heading);
	if (start === -1) return [];
	const section = content.slice(start).split(/\n## /)[0];
	const header = section.split("\n").find((line) => line.startsWith("|") && !line.includes("---"));
	return header ? header.split("|").slice(1, -1).map((cell) => cell.trim()) : [];
}

function validateTable() {
	const header = tableHeaderAfter("## Debt Review");
	if (header.join("|") !== columns.join("|")) errors.push(`Debt Review: columns must be ${columns.join(", ")}`);
	if (rows.length < 1) errors.push("Debt Review: at least one review row is required");
	for (const row of rows) {
		if (row.length !== columns.length) errors.push(`Debt Review: row ${row[0] ?? "unknown"} must have ${columns.length} columns`);
	}
}

function validateDebtRow(row) {
	const [item, owner, scope, v1Required, evidence, status, decision] = row;
	if (!item) errors.push("debt item name is required");
	if (["", "pending", "none"].includes(owner)) errors.push(`${item}: owner is required`);
	if (!["V1", "V2", "V1+V2"].includes(scope)) errors.push(`${item}: scope must be V1, V2 or V1+V2`);
	if (!["yes", "no"].includes(v1Required)) errors.push(`${item}: V1 Required must be yes or no`);
	if (!["open", "closed", "accepted", "removed"].includes(status)) errors.push(`${item}: unsupported status ${status}`);
	if (!["go", "no-go"].includes(decision)) errors.push(`${item}: unsupported decision ${decision}`);
	if (isYes(v1Required) && decision === "go" && status !== "closed") {
		errors.push(`${item}: V1-required go decision requires closed status`);
	}
	if (decision === "go" && !hasConcreteEvidence(evidence)) {
		errors.push(`${item}: go decision requires concrete evidence`);
	}
	if (status === "open" && decision !== "no-go") errors.push(`${item}: open status requires no-go decision`);
}

function validateStatusSummary() {
	const v1RequiredRows = rows.filter((row) => isYes(row[3])).length;
	const openV1RequiredRows = rows.filter((row) => isYes(row[3]) && row[5] !== "closed").length;
	const noGoRows = rows.filter((row) => row[6] === "no-go").length;
	const expected = {
		total_rows: rows.length,
		v1_required_rows: v1RequiredRows,
		open_v1_required_rows: openV1RequiredRows,
		no_go_rows: noGoRows,
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

function isYes(value) {
	return value === "yes";
}

function hasConcreteEvidence(value) {
	return !["", "pending", "none", "no-go"].includes(value ?? "") && /\b(audit|backlog|query|report|review|ticket|evidence|acceptance)\b/i.test(value);
}

if (errors.length > 0) {
	console.error("V2 debt register checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(`V2 debt register: ok (${rows.length} rows)`);
