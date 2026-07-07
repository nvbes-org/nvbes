#!/usr/bin/env node
import { readFileSync } from "node:fs";

const args = process.argv.slice(2);
const strict = args.includes("--strict");
const path = "docs/migration/communication-plan.md";
const content = readFileSync(path, "utf8");

const requiredHeadings = [
	"# Migration Communication Plan",
	"## Status",
	"## Rules",
	"## Audiences",
	"## Message Templates",
	"## Approval Checklist",
	"## Decision",
];

const requiredAudiences = ["customers", "internal responders", "product owners", "support team"];

const requiredMessages = [
	"maintenance announcement",
	"maintenance started",
	"progress update",
	"rollback notice",
	"service restored",
	"post-cutover hypercare",
];

const requiredChecks = [
	"status page ready",
	"customer impact described",
	"rollback message ready",
	"support escalation route",
	"final success criteria",
];

const audienceColumns = ["Audience", "Channel", "Owner", "Required Timing", "Status"];
const messageColumns = ["Message", "Trigger", "Approver", "Evidence", "Decision"];
const approvalColumns = ["Check", "Required State", "Current State"];
const validAudienceStatuses = ["pending", "ready", "sent", "accepted", "failed", "blocking"];
const validApprovalStates = ["pending", "ready", "passed", "accepted"];
const errors = [];

for (const heading of requiredHeadings) {
	if (!content.includes(heading)) errors.push(`missing heading: ${heading}`);
}

for (const audience of requiredAudiences) {
	if (!content.includes(`| ${audience} |`)) errors.push(`missing audience: ${audience}`);
}

for (const message of requiredMessages) {
	if (!content.includes(`| ${message} |`)) errors.push(`missing message template: ${message}`);
}

for (const check of requiredChecks) {
	if (!content.includes(`| ${check} |`)) errors.push(`missing approval check: ${check}`);
}

validateTable("## Audiences", audienceColumns, requiredAudiences);
validateTable("## Message Templates", messageColumns, requiredMessages);
validateTable("## Approval Checklist", approvalColumns, requiredChecks);
validateStatusSummary();

const approvalRows = tableRowsAfter("## Approval Checklist");

for (const row of tableRowsAfter("## Audiences")) {
	validateAudienceRow(row);
}
for (const row of tableRowsAfter("## Message Templates")) {
	validateMessageRow(row);
}
for (const row of approvalRows) {
	validateApprovalRow(row);
}

if (strict) {
	const blockers = strictBlockers();
	for (const [blocker, count] of Object.entries(blockers)) {
		if (count > 0) errors.push(`${count} ${blocker} communication marker(s) remain`);
	}
}

if (!content.includes("Current decision: no-go.")) errors.push("missing current communication no-go decision");
if (!content.includes("Communication is not approved until the cutover window")) {
	errors.push("missing communication approval blocker");
}

function tableRowsAfter(heading) {
	const start = content.indexOf(heading);
	if (start === -1) return [];
	const section = content.slice(start).split(/\n## /)[0];
	return section
		.split("\n")
		.filter((line) => line.startsWith("|") && !line.includes("---") && !line.includes("Audience |") && !line.includes("Message |") && !line.includes("Check |"))
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

function validateAudienceRow(row) {
	const [audience, channel, owner, timing, status] = row;
	if (!validAudienceStatuses.includes(status)) {
		errors.push(`${audience}: unsupported audience status ${status}`);
	}
	if (["ready", "sent", "accepted"].includes(status)) {
		for (const [field, value] of [["channel", channel], ["owner", owner], ["required timing", timing]]) {
			if (isEmptyMarker(value)) errors.push(`${audience}: ${status} audience requires ${field}`);
		}
	}
}

function validateMessageRow(row) {
	const [message, trigger, approver, evidence, decision] = row;
	if (!["go", "no-go"].includes(decision)) errors.push(`${message}: unsupported message decision ${decision}`);
	if (decision === "go") {
		for (const [field, value] of [["trigger", trigger], ["approver", approver], ["evidence", evidence]]) {
			if (isEmptyMarker(value)) errors.push(`${message}: go decision requires ${field}`);
		}
		if (!hasConcreteEvidence(evidence)) {
			errors.push(`${message}: go decision requires concrete communication evidence`);
		}
		for (const [check, , currentState] of approvalRows) {
			if (!["passed", "accepted"].includes(currentState)) {
				errors.push(`${message}: go decision requires ${check} approval check`);
			}
		}
	}
}

function validateApprovalRow(row) {
	const [check, requiredState, currentState] = row;
	if (!validApprovalStates.includes(currentState)) {
		errors.push(`${check}: unsupported approval state ${currentState}`);
	}
	if (["passed", "accepted"].includes(currentState) && !hasConcreteEvidence(requiredState)) {
		errors.push(`${check}: passed/accepted approval requires concrete required-state evidence`);
	}
}

function validateStatusSummary() {
	const audiences = tableRowsAfter("## Audiences");
	const messages = tableRowsAfter("## Message Templates");
	const approvals = tableRowsAfter("## Approval Checklist");
	const audienceCounts = countColumn(audiences, -1, validAudienceStatuses);
	const decisionCounts = countColumn(messages, -1, ["go", "no-go"]);
	const approvalCounts = countColumn(approvals, -1, validApprovalStates);
	const expected = {
		total_audience_rows: audiences.length,
		pending_audience_rows: audienceCounts.pending,
		ready_audience_rows: audienceCounts.ready,
		sent_audience_rows: audienceCounts.sent,
		accepted_audience_rows: audienceCounts.accepted,
		failed_audience_rows: audienceCounts.failed,
		blocking_audience_rows: audienceCounts.blocking,
		total_message_rows: messages.length,
		go_decisions: decisionCounts.go,
		no_go_decisions: decisionCounts["no-go"],
		total_approval_rows: approvals.length,
		pending_approval_rows: approvalCounts.pending,
		ready_approval_rows: approvalCounts.ready,
		passed_approval_rows: approvalCounts.passed,
		accepted_approval_rows: approvalCounts.accepted,
	};
	for (const [key, value] of Object.entries(expected)) {
		const actual = statusNumber(key);
		if (actual === null) errors.push(`missing status summary: ${key}`);
		else if (actual !== value) errors.push(`${key}: expected ${value}, found ${actual}`);
	}
}

function countColumn(rows, column, values) {
	const counts = Object.fromEntries(values.map((value) => [value, 0]));
	for (const row of rows) counts[row.at(column)] = (counts[row.at(column)] ?? 0) + 1;
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
	const normalized = (value ?? "").toLowerCase();
	return /(artifact|link|url|status page|message|email|approval|record|journal|incident|route|owner|tested|signed|criteria|smoke|reconciliation|report|result)/.test(normalized);
}

function strictBlockers() {
	const counts = { pending: 0, blocking: 0, failed: 0, "no-go": 0 };
	for (const row of tableRowsAfter("## Audiences")) {
		const status = row.at(-1);
		if (status === "pending") counts.pending += 1;
		if (status === "blocking") counts.blocking += 1;
		if (status === "failed") counts.failed += 1;
	}
	for (const row of tableRowsAfter("## Message Templates")) {
		if (row.at(-1) === "no-go") counts["no-go"] += 1;
	}
	for (const row of tableRowsAfter("## Approval Checklist")) {
		if (row.at(-1) === "pending") counts.pending += 1;
	}
	return counts;
}

if (errors.length > 0) {
	console.error("Communication checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(
	`Communication: ok (${requiredAudiences.length} audiences, ${requiredMessages.length} messages)`,
);
