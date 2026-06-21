#!/usr/bin/env node
import { readFileSync } from "node:fs";

const args = process.argv.slice(2);
const strict = args.includes("--strict");
const path = "docs/migration/decommission-manifest.md";
const content = readFileSync(path, "utf8");

const requiredHeadings = [
	"# Decommission Manifest",
	"## Status",
	"## Rules",
	"## Legacy Runtime",
	"## Final Evidence",
	"## Decision",
];

const requiredRuntime = [
	"legacy API services",
	"legacy web frontends",
	"legacy workers",
	"legacy queues",
	"legacy schedules",
	"legacy storage paths",
	"legacy configs",
	"legacy scripts",
];

const requiredEvidence = [
	"legacy secrets revoked",
	"OSS export clean",
	"boundaries clean",
	"migration exports archived",
	"support hypercare closed",
	"runbooks updated",
	"V2 backlog clean",
	"post-migration audit signed",
];

const runtimeColumns = ["Component", "Owner", "Evidence", "Status", "Decision"];
const evidenceColumns = ["Evidence", "Owner", "Required content", "Status", "Decision"];
const validStatuses = ["pending", "passed", "accepted", "removed", "failed", "blocking"];
const validDecisions = ["go", "no-go"];
const errors = [];

for (const heading of requiredHeadings) {
	if (!content.includes(heading)) errors.push(`missing heading: ${heading}`);
}

for (const runtime of requiredRuntime) {
	if (!content.includes(`| ${runtime} |`)) errors.push(`missing legacy runtime row: ${runtime}`);
}

for (const evidence of requiredEvidence) {
	if (!content.includes(`| ${evidence} |`)) errors.push(`missing final evidence row: ${evidence}`);
}

validateTable("## Legacy Runtime", runtimeColumns, requiredRuntime);
validateTable("## Final Evidence", evidenceColumns, requiredEvidence);
validateStatusSummary();

const runtimeRows = tableRowsAfter("## Legacy Runtime");

for (const row of runtimeRows) {
	validateRuntimeRow(row);
}
for (const row of tableRowsAfter("## Final Evidence")) {
	validateFinalEvidenceRow(row);
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
			errors.push(`${count} ${blocker} decommission marker(s) remain`);
		}
	}
}

if (!content.includes("The migration goal remains incomplete until every decommission row is evidenced")) {
	errors.push("missing explicit decommission blocker");
}

function tableRowsAfter(heading) {
	const start = content.indexOf(heading);
	if (start === -1) return [];
	const section = content.slice(start).split(/\n## /)[0];
	return section
		.split("\n")
		.filter((line) => line.startsWith("|") && !line.includes("---") && !line.includes("Component") && !line.includes("Evidence |"))
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

function validateRuntimeRow(row) {
	const [component, owner, evidence, status, decision] = row;
	if (["pending", "none", ""].includes(owner)) errors.push(`${component}: owner is required`);
	if (decision === "go" && ["pending", "none", ""].includes(evidence)) {
		errors.push(`${component}: go decision requires runtime evidence`);
	}
	if (decision === "go" && !hasConcreteRuntimeEvidence(evidence)) {
		errors.push(`${component}: go decision requires concrete runtime removal evidence`);
	}
	validateDecisionRow(component, status, decision);
}

function validateFinalEvidenceRow(row) {
	const [evidence, owner, requiredContent, status, decision] = row;
	validateDecisionRow(evidence, status, decision);
	if (decision !== "go") return;
	if (["pending", "none", ""].includes(owner)) errors.push(`${evidence}: go decision requires owner`);
	if (!hasConcreteEvidence(requiredContent)) errors.push(`${evidence}: go decision requires concrete required content`);
	for (const [component, , , runtimeStatus] of runtimeRows) {
		if (!["passed", "accepted", "removed"].includes(runtimeStatus)) {
			errors.push(`${evidence}: go decision requires ${component} runtime row`);
		}
	}
}

function validateDecisionRow(label, status, decision) {
	if (!validStatuses.includes(status)) errors.push(`${label}: unsupported decommission status ${status}`);
	if (!validDecisions.includes(decision)) errors.push(`${label}: unsupported decommission decision ${decision}`);
	if (decision === "go" && !["passed", "accepted", "removed"].includes(status)) {
		errors.push(`${label}: go decision requires passed, accepted or removed status`);
	}
	if (["pending", "blocking", "failed"].includes(status) && decision !== "no-go") {
		errors.push(`${label}: ${status} status requires no-go decision`);
	}
}

function validateStatusSummary() {
	const rows = [...tableRowsAfter("## Legacy Runtime"), ...tableRowsAfter("## Final Evidence")];
	const statusCounts = countValues(rows.map((row) => row.at(-2)), validStatuses);
	const decisionCounts = countValues(rows.map((row) => row.at(-1)), validDecisions);
	const expected = {
		total_decision_rows: rows.length,
		pending_rows: statusCounts.pending,
		passed_rows: statusCounts.passed,
		accepted_rows: statusCounts.accepted,
		removed_rows: statusCounts.removed,
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
	return !["", "pending", "none", "no-go"].includes(value ?? "") && /\b(log|report|result|retention|encryption|expiry|docs|links|audit|debt|incident|check)\b/i.test(value);
}

function hasConcreteRuntimeEvidence(value) {
	return !["", "pending", "none", "no-go"].includes(value ?? "") && /\b(removal|removed|archive|archived|disabled|deleted|inventory|report|log|record|manifest|retention|link|url|id|check)\b/i.test(value);
}

if (errors.length > 0) {
	console.error("Decommission checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(
	`Decommission: ok (${requiredRuntime.length} runtime rows, ${requiredEvidence.length} evidence rows)`,
);
