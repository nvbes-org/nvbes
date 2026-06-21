#!/usr/bin/env node
import { readFileSync } from "node:fs";

const args = process.argv.slice(2);
const strict = args.includes("--strict");
const path = "docs/migration/owner-signoff-matrix.md";
const gateEvidencePath = "docs/migration/gate-evidence.generated.json";
const content = readFileSync(path, "utf8");

const requiredHeadings = [
	"# Owner Sign-Off Matrix",
	"## Status",
	"## Rules",
	"## Required Owners",
	"## Source Class Sign-Offs",
	"## Gate Sign-Offs",
	"## Decision",
];

const requiredAreas = [
	"Migration lead",
	"Data lead",
	"Infra lead",
	"Security lead",
	"Support lead",
	"Identity owner",
	"Workspace/Authz owner",
	"Drive owner",
	"Billing/Usage owner",
	"Audit/Privacy owner",
	"Developer Platform owner",
	"Cloud/Internal owner",
	"Contracts owner",
	"OSS export owner",
];

const requiredSourceClasses = [
	"routes and endpoints",
	"source tables",
	"secrets",
	"jobs and workers",
	"infra resources",
	"blocking risks",
	"rejected data",
	"rollback path",
];

const requiredGates = [
	"G0 Freeze",
	"G1 Fondation",
	"G2 Primitives",
	"G3 Domaines",
	"G4 Frontends",
	"G5 Infra",
	"G6 Repetitions",
	"G7 Cutover",
	"G8 Decommission",
];

const ownerColumns = ["Area", "Scope", "Primary owner", "Backup owner", "Evidence", "Status", "Decision"];
const sourceColumns = ["Source class", "Decision owner", "Evidence", "Status", "Decision"];
const gateColumns = ["Gate", "Owner", "Evidence", "Status", "Decision"];
const errors = [];
const gateEvidence = readJson(gateEvidencePath);

for (const heading of requiredHeadings) {
	if (!content.includes(heading)) errors.push(`missing heading: ${heading}`);
}

for (const area of requiredAreas) {
	if (!content.includes(`| ${area} |`)) errors.push(`missing required owner area: ${area}`);
}

for (const sourceClass of requiredSourceClasses) {
	if (!content.includes(`| ${sourceClass} |`)) {
		errors.push(`missing source class sign-off: ${sourceClass}`);
	}
}

for (const gate of requiredGates) {
	if (!content.includes(`| ${gate} |`)) errors.push(`missing gate sign-off: ${gate}`);
}

validateTable("## Required Owners", ownerColumns, requiredAreas);
validateTable("## Source Class Sign-Offs", sourceColumns, requiredSourceClasses);
validateTable("## Gate Sign-Offs", gateColumns, requiredGates);
validateStatusSummary();

for (const row of tableRowsAfter("## Required Owners")) {
	validateRequiredOwner(row);
}

for (const row of tableRowsAfter("## Source Class Sign-Offs")) {
	validateSignOffRow(row[0], row[1], row[2], row.at(-2), row.at(-1));
}

for (const row of tableRowsAfter("## Gate Sign-Offs")) {
	validateSignOffRow(row[0], row[1], row[2], row.at(-2), row.at(-1));
	validateGateSignOff(row);
}

if (strict) {
	const blockers = [...content.matchAll(/\b(unassigned|pending|no-go)\b/gi)].map(
		(match) => match[1].toLowerCase(),
	);
	if (blockers.length > 0) {
		const counts = blockers.reduce((acc, blocker) => {
			acc[blocker] = (acc[blocker] ?? 0) + 1;
			return acc;
		}, {});
		for (const [blocker, count] of Object.entries(counts)) {
			errors.push(`${count} ${blocker} owner sign-off marker(s) remain`);
		}
	}
}

if (!content.includes("No production cutover can be approved until all required owners")) {
	errors.push("missing explicit owner sign-off blocker");
}

function tableRowsAfter(heading) {
	const start = content.indexOf(heading);
	if (start === -1) return [];
	const section = content.slice(start).split(/\n## /)[0];
	return section
		.split("\n")
		.filter((line) => line.startsWith("|") && !line.includes("---"))
		.filter((line) => !line.includes("Area |") && !line.includes("Source class |") && !line.includes("Gate |"))
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

function validateRequiredOwner(row) {
	const [area, , primaryOwner, backupOwner, evidence, status, decision] = row;
	validateStatusDecision(area, status, decision);
	if (decision === "go" || status === "signed") {
		for (const [field, value] of [
			["primary owner", primaryOwner],
			["backup owner", backupOwner],
			["evidence", evidence],
		]) {
			if (isEmptyMarker(value)) errors.push(`${area}: signed/go sign-off requires ${field}`);
		}
		if (!hasConcreteEvidence(evidence)) {
			errors.push(`${area}: signed/go sign-off requires concrete evidence`);
		}
		if (!isEmptyMarker(primaryOwner) && primaryOwner === backupOwner) {
			errors.push(`${area}: primary owner and backup owner must differ`);
		}
	}
}

function validateSignOffRow(label, owner, evidence, status, decision) {
	validateStatusDecision(label, status, decision);
	if (decision === "go" || status === "signed") {
		if (isEmptyMarker(owner)) errors.push(`${label}: signed/go sign-off requires owner`);
		if (isEmptyMarker(evidence)) errors.push(`${label}: signed/go sign-off requires evidence`);
		if (!hasConcreteEvidence(evidence)) {
			errors.push(`${label}: signed/go sign-off requires concrete evidence`);
		}
	}
}

function validateGateSignOff(row) {
	const [gate, , , status, decision] = row;
	if (status !== "signed" && decision !== "go") return;
	const gateRow = gateEvidence?.gates?.find((entry) => entry.gate === gate);
	if (!gateRow) {
		errors.push(`${gate}: signed/go gate sign-off requires generated gate evidence`);
		return;
	}
	if (gateRow.decision !== "go") {
		errors.push(`${gate}: signed/go gate sign-off requires gate evidence go decision`);
	}
}

function validateStatusDecision(label, status, decision) {
	if (!["pending", "signed"].includes(status)) {
		errors.push(`${label}: unsupported sign-off status ${status}`);
	}
	if (!["no-go", "go"].includes(decision)) {
		errors.push(`${label}: unsupported sign-off decision ${decision}`);
	}
	if (status === "pending" && decision !== "no-go") {
		errors.push(`${label}: pending sign-off requires no-go decision`);
	}
	if (decision === "go" && status !== "signed") {
		errors.push(`${label}: go decision requires signed status`);
	}
}

function isEmptyMarker(value) {
	return ["", "pending", "unassigned", "none", "no-go"].includes(value ?? "");
}

function hasConcreteEvidence(value) {
	return /(artifact|report|result|generated|ledger|matrix|journal|record|sign|approval|link|url|sha|digest|snapshot|reconciliation|rollback|gate evidence)/i.test(value ?? "");
}

function readJson(jsonPath) {
	try {
		return JSON.parse(readFileSync(jsonPath, "utf8"));
	} catch (error) {
		errors.push(`${jsonPath}: invalid or missing JSON: ${error.message}`);
		return undefined;
	}
}

function validateStatusSummary() {
	const counts = signOffCounts();
	for (const [key, expected] of Object.entries(counts)) {
		const actual = statusNumber(key);
		if (actual === undefined) errors.push(`missing status summary: ${key}`);
		else if (actual !== expected) errors.push(`${key}: expected ${expected}, found ${actual}`);
	}
}

function signOffCounts() {
	const requiredOwnerRows = tableRowsAfter("## Required Owners");
	const sourceRows = tableRowsAfter("## Source Class Sign-Offs");
	const gateRows = tableRowsAfter("## Gate Sign-Offs");
	const rows = [...requiredOwnerRows, ...sourceRows, ...gateRows];
	return {
		total_signoff_rows: rows.length,
		signed_rows: rows.filter((row) => row.at(-2) === "signed").length,
		go_decisions: rows.filter((row) => row.at(-1) === "go").length,
		pending_rows: rows.filter((row) => row.at(-2) === "pending").length,
		no_go_decisions: rows.filter((row) => row.at(-1) === "no-go").length,
		unassigned_owner_fields: countUnassignedOwners(requiredOwnerRows, sourceRows, gateRows),
	};
}

function countUnassignedOwners(requiredOwnerRows, sourceRows, gateRows) {
	const requiredOwnerFields = requiredOwnerRows.flatMap((row) => [row[2], row[3]]);
	const sourceOwnerFields = sourceRows.map((row) => row[1]);
	const gateOwnerFields = gateRows.map((row) => row[1]);
	return [...requiredOwnerFields, ...sourceOwnerFields, ...gateOwnerFields].filter((value) => value === "unassigned").length;
}

function statusNumber(key) {
	const match = content.match(new RegExp(`^- ${key}: (\\d+)$`, "m"));
	return match ? Number.parseInt(match[1], 10) : undefined;
}

if (errors.length > 0) {
	console.error("Owner sign-off checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(`Owner sign-offs: ok (${requiredAreas.length} areas, ${requiredGates.length} gates)`);
