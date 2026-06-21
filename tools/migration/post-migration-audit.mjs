#!/usr/bin/env node
import { readFileSync } from "node:fs";

const args = process.argv.slice(2);
const strict = args.includes("--strict");
const path = "docs/migration/post-migration-audit.md";
const content = readFileSync(path, "utf8");

const requiredHeadings = [
	"# Post-Migration Audit",
	"## Status",
	"## Rules",
	"## Required Sign-Off",
	"## Audit Items",
	"## Decision",
];

const requiredOwners = [
	"Migration lead",
	"Security lead",
	"Data lead",
	"Product leads",
	"Infra lead",
];

const requiredItems = [
	"no legacy runtime service active",
	"no legacy job active",
	"no unused legacy secret active",
	"no private doc in OSS export",
	"no provider boundary violation",
	"backups and restore verified",
	"hypercare complete",
];

const signoffColumns = ["Owner", "Scope", "Status"];
const auditColumns = ["Item", "Evidence", "Status"];
const validSignoffStatuses = ["pending", "approved", "accepted", "failed", "blocking"];
const validAuditStatuses = ["pending", "approved", "accepted", "passed", "failed", "blocking"];
const errors = [];

for (const heading of requiredHeadings) {
	if (!content.includes(heading)) errors.push(`missing heading: ${heading}`);
}

for (const owner of requiredOwners) {
	if (!content.includes(`| ${owner} |`)) errors.push(`missing audit sign-off: ${owner}`);
}

for (const item of requiredItems) {
	if (!content.includes(`| ${item} |`)) errors.push(`missing audit item: ${item}`);
}

validateTable("## Required Sign-Off", signoffColumns, requiredOwners);
validateTable("## Audit Items", auditColumns, requiredItems);
validateStatusSummary();

const auditItemRows = tableRowsAfter("## Audit Items");

for (const [owner, scope, status] of tableRowsAfter("## Required Sign-Off")) {
	if (!validSignoffStatuses.includes(status)) errors.push(`${owner}: unsupported sign-off status ${status}`);
	if (["approved", "accepted"].includes(status) && ["pending", "none", ""].includes(scope)) {
		errors.push(`${owner}: approved sign-off requires scope`);
	}
	if (["approved", "accepted"].includes(status) && !hasConcreteEvidence(scope)) {
		errors.push(`${owner}: approved sign-off requires concrete evidence reference in scope`);
	}
	if (["approved", "accepted"].includes(status)) {
		for (const [item, , auditStatus] of auditItemRows) {
			if (!["approved", "accepted", "passed"].includes(auditStatus)) {
				errors.push(`${owner}: approved sign-off requires ${item} audit item`);
			}
		}
	}
}
for (const [item, evidence, status] of auditItemRows) {
	if (!validAuditStatuses.includes(status)) errors.push(`${item}: unsupported audit item status ${status}`);
	if (["approved", "accepted", "passed"].includes(status) && ["pending", "none", ""].includes(evidence)) {
		errors.push(`${item}: approved audit item requires evidence`);
	}
	if (["approved", "accepted", "passed"].includes(status) && !hasConcreteEvidence(evidence)) {
		errors.push(`${item}: approved audit item requires concrete evidence`);
	}
}

if (!content.includes("Decommission is not complete until every audit item is approved.")) {
	errors.push("missing post-migration audit completion blocker");
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
			errors.push(`${count} ${blocker} post-migration audit marker(s) remain`);
		}
	}
}

function tableRowsAfter(heading) {
	const start = content.indexOf(heading);
	if (start === -1) return [];
	const section = content.slice(start).split(/\n## /)[0];
	return section
		.split("\n")
		.filter((line) => line.startsWith("|") && !line.includes("---") && !line.includes("Owner |") && !line.includes("Item |"))
		.map((line) => line.split("|").slice(1, -1).map((cell) => cell.trim().replace(/^`|`$/g, "")));
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

function validateStatusSummary() {
	const signoffs = tableRowsAfter("## Required Sign-Off");
	const auditItems = tableRowsAfter("## Audit Items");
	const signoffStatuses = countValues(signoffs.map((row) => row.at(-1)), validSignoffStatuses);
	const auditStatuses = countValues(auditItems.map((row) => row.at(-1)), validAuditStatuses);
	const expected = {
		total_signoff_rows: signoffs.length,
		pending_signoff_rows: signoffStatuses.pending,
		approved_signoff_rows: signoffStatuses.approved,
		accepted_signoff_rows: signoffStatuses.accepted,
		failed_signoff_rows: signoffStatuses.failed,
		blocking_signoff_rows: signoffStatuses.blocking,
		total_audit_rows: auditItems.length,
		pending_audit_rows: auditStatuses.pending,
		approved_audit_rows: auditStatuses.approved,
		accepted_audit_rows: auditStatuses.accepted,
		passed_audit_rows: auditStatuses.passed,
		failed_audit_rows: auditStatuses.failed,
		blocking_audit_rows: auditStatuses.blocking,
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
	return /\b(docs|tools|pnpm|artifact|ci|s3|gs|oci):?\/?\/?/.test(value ?? "") || /\b(report|journal|inventory|log|evidence|check)\b/i.test(value ?? "");
}

if (errors.length > 0) {
	console.error("Post-migration audit checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(
	`Post-migration audit: ok (${requiredOwners.length} sign-offs, ${requiredItems.length} audit items)`,
);
