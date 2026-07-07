#!/usr/bin/env node
import { existsSync, readFileSync } from "node:fs";

const args = process.argv.slice(2);
const strict = args.includes("--strict");
const path = "docs/migration/v2-debt-register.md";
const reviewPath = "docs/migration/v2-debt-review.md";
const content = readFileSync(path, "utf8");
const reviewContent = existsSync(reviewPath) ? readFileSync(reviewPath, "utf8") : "";
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
validateDebtReviewEvidence();
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
	return tableRowsIn(content, heading);
}

function tableRowsIn(source, heading) {
	const start = source.indexOf(heading);
	if (start === -1) return [];
	const section = source.slice(start).split(/\n## /)[0];
	return section
		.split("\n")
		.filter((line) => line.startsWith("|") && !line.includes("---") && !line.includes("Item |"))
		.map((line) => line.split("|").slice(1, -1).map((cell) => cell.trim()))
		.filter((row) => !["Item", "Finding"].includes(row[0]));
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
	validateEvidencePaths(item, evidence);
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

function validateDebtReviewEvidence() {
	if (!existsSync(reviewPath)) {
		errors.push(`${reviewPath}: missing`);
		return;
	}
	for (const heading of ["# V2 Debt Review", "## Status", "## Findings", "## Resolved Findings", "## Decision"]) {
		if (!reviewContent.includes(heading)) errors.push(`${reviewPath}: missing heading: ${heading}`);
	}
	const findings = tableRowsIn(reviewContent, "## Findings");
	const resolvedFindings = tableRowsIn(reviewContent, "## Resolved Findings");
	const expected = {
		v1_required_findings: findings.length,
		resolved_findings: resolvedFindings.length,
	};
	for (const [key, value] of Object.entries(expected)) {
		const actual = reviewStatusNumber(key);
		if (actual === null) errors.push(`${reviewPath}: missing status summary: ${key}`);
		else if (actual !== value) errors.push(`${reviewPath}: ${key} expected ${value}, found ${actual}`);
	}
	if (reviewStatusValue("owner_acceptance") === "missing" && reviewStatusValue("decision") === "go") {
		errors.push(`${reviewPath}: go decision requires owner acceptance`);
	}
	for (const row of findings) {
		if (row.length !== 4) errors.push(`${reviewPath}: finding row ${row[0] ?? "unknown"} must have 4 columns`);
	}
	for (const row of resolvedFindings) {
		if (row.length !== 3) errors.push(`${reviewPath}: resolved finding row ${row[0] ?? "unknown"} must have 3 columns`);
	}
}

function statusNumber(key) {
	const match = content.match(new RegExp(`- \`${key}\`: (\\d+)`));
	return match ? Number.parseInt(match[1], 10) : null;
}

function reviewStatusNumber(key) {
	const value = reviewStatusValue(key);
	return value && /^\d+$/.test(value) ? Number.parseInt(value, 10) : null;
}

function reviewStatusValue(key) {
	const match = reviewContent.match(new RegExp(`- ${key}: ([^\\n]+)`));
	return match ? match[1].trim() : null;
}

function isYes(value) {
	return value === "yes";
}

function hasConcreteEvidence(value) {
	return !["", "pending", "none", "no-go"].includes(value ?? "") && /\b(audit|backlog|query|report|review|ticket|evidence|acceptance)\b/i.test(value);
}

function validateEvidencePaths(item, evidence) {
	const matches = evidence.match(/docs\/migration\/[a-z0-9./_-]+/g) ?? [];
	for (const path of matches) {
		if (!existsSync(path)) errors.push(`${item}: evidence path missing: ${path}`);
	}
}

if (errors.length > 0) {
	console.error("V2 debt register checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(`V2 debt register: ok (${rows.length} rows)`);
