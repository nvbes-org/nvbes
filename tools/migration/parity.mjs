#!/usr/bin/env node
import { existsSync, readFileSync } from "node:fs";

const args = process.argv.slice(2);
const strict = args.includes("--strict");
const path = "docs/migration/parity-matrix.md";
const content = readFileSync(path, "utf8");
const identityRegisterEvidencePath = "docs/migration/identity-register.generated.json";
const identityLoginSessionEvidencePath = "docs/migration/identity-login-session.generated.json";
const identityMfaWebauthnEvidencePath = "docs/migration/identity-mfa-webauthn.generated.json";
const auditEvidencePath = "docs/migration/audit-append-only.generated.json";
const workspaceMembershipRolesEvidencePath = "docs/migration/workspace-membership-roles.generated.json";
const workspaceLastOwnerEvidencePath = "docs/migration/workspace-last-owner.generated.json";
const driveUploadDownloadEvidencePath = "docs/migration/drive-upload-download.generated.json";
const driveShareRevokeEvidencePath = "docs/migration/drive-share-revoke.generated.json";
const driveQuotasEvidencePath = "docs/migration/drive-quotas.generated.json";
const privacyExportDeleteEvidencePath = "docs/migration/privacy-export-delete.generated.json";
const billingEntitlementsEvidencePath = "docs/migration/billing-entitlements.generated.json";
const billingWebhookEvidencePath = "docs/migration/billing-webhook-idempotency.generated.json";
const developerOauthTokenEvidencePath = "docs/migration/developer-oauth-tokens.generated.json";
const developerSignedWebhookEvidencePath = "docs/migration/developer-signed-webhooks.generated.json";
const cloudProvisioningEvidencePath = "docs/migration/cloud-provisioning.generated.json";

const requiredHeadings = [
	"# Functional Parity Matrix",
	"## Status",
	"## Gate",
	"## Matrix",
	"## Review Rules",
];

const requiredCapabilities = [
	"Identity | register",
	"Identity | login/logout/refresh",
	"Identity | MFA/WebAuthn",
	"Workspace/Authz | membership roles",
	"Workspace/Authz | last-owner protection",
	"Drive | upload/download",
	"Drive | share/revoke",
	"Drive | quotas",
	"Billing/Usage | entitlements",
	"Billing/Usage | webhooks",
	"Audit/Privacy | audit append-only",
	"Audit/Privacy | export/delete requests",
	"Developer Platform | OAuth apps/tokens",
	"Developer Platform | signed webhooks",
	"Cloud/Internal | provisioning console",
];

const requiredStatuses = ["pending", "covered", "accepted reject", "removed"];
const requiredColumns = ["Domain", "Capability", "Critical", "Owner", "Target evidence", "Status"];
const errors = [];

for (const heading of requiredHeadings) {
	if (!content.includes(heading)) errors.push(`missing heading: ${heading}`);
}

for (const capability of requiredCapabilities) {
	if (!content.includes(`| ${capability} |`)) {
		errors.push(`missing parity capability: ${capability}`);
	}
}

for (const status of requiredStatuses) {
	if (!content.includes(status)) errors.push(`missing review status: ${status}`);
}

for (const column of requiredColumns) {
	if (!content.includes(column)) errors.push(`missing parity matrix column: ${column}`);
}

validateMatrixShape();
validateStatusSummary();

for (const row of tableRowsAfter("## Matrix")) {
	validateParityRow(row);
}

if (strict) {
	const pendingRows = content
		.split("\n")
		.filter((line) => line.startsWith("|") && line.includes("| yes |") && /\| pending \|$/.test(line));
	if (pendingRows.length > 0) {
		errors.push(`${pendingRows.length} critical parity row(s) remain pending`);
	}
}

const identityRegisterCovered = content
	.split("\n")
	.some((line) => line.startsWith("| Identity | register |") && /\| covered \|$/.test(line));
if (identityRegisterCovered) {
	requirePassedEvidence(identityRegisterEvidencePath, "covered identity register parity");
}

const identityLoginSessionCovered = content
	.split("\n")
	.some((line) => line.startsWith("| Identity | login/logout/refresh |") && /\| covered \|$/.test(line));
if (identityLoginSessionCovered) {
	requirePassedEvidence(identityLoginSessionEvidencePath, "covered identity login/logout/refresh parity");
}

const identityMfaWebauthnCovered = content
	.split("\n")
	.some((line) => line.startsWith("| Identity | MFA/WebAuthn |") && /\| covered \|$/.test(line));
if (identityMfaWebauthnCovered) {
	requirePassedEvidence(identityMfaWebauthnEvidencePath, "covered identity MFA/WebAuthn parity");
}

const auditAppendOnlyCovered = content
	.split("\n")
	.some((line) => line.startsWith("| Audit/Privacy | audit append-only |") && /\| covered \|$/.test(line));
if (auditAppendOnlyCovered) {
	requirePassedEvidence(auditEvidencePath, "covered audit append-only parity");
}

const workspaceLastOwnerCovered = content
	.split("\n")
	.some((line) => line.startsWith("| Workspace/Authz | last-owner protection |") && /\| covered \|$/.test(line));
if (workspaceLastOwnerCovered) {
	requirePassedEvidence(workspaceLastOwnerEvidencePath, "covered workspace last-owner parity");
}

const workspaceMembershipRolesCovered = content
	.split("\n")
	.some((line) => line.startsWith("| Workspace/Authz | membership roles |") && /\| covered \|$/.test(line));
if (workspaceMembershipRolesCovered) {
	requirePassedEvidence(workspaceMembershipRolesEvidencePath, "covered workspace membership role parity");
}

const privacyExportDeleteCovered = content
	.split("\n")
	.some((line) => line.startsWith("| Audit/Privacy | export/delete requests |") && /\| covered \|$/.test(line));
if (privacyExportDeleteCovered) {
	requirePassedEvidence(privacyExportDeleteEvidencePath, "covered privacy export/delete parity");
}

const driveQuotasCovered = content
	.split("\n")
	.some((line) => line.startsWith("| Drive | quotas |") && /\| covered \|$/.test(line));
if (driveQuotasCovered) {
	requirePassedEvidence(driveQuotasEvidencePath, "covered drive quotas parity");
}

const driveUploadDownloadCovered = content
	.split("\n")
	.some((line) => line.startsWith("| Drive | upload/download |") && /\| covered \|$/.test(line));
if (driveUploadDownloadCovered) {
	requirePassedEvidence(driveUploadDownloadEvidencePath, "covered drive upload/download parity");
}

const driveShareRevokeCovered = content
	.split("\n")
	.some((line) => line.startsWith("| Drive | share/revoke |") && /\| covered \|$/.test(line));
if (driveShareRevokeCovered) {
	requirePassedEvidence(driveShareRevokeEvidencePath, "covered drive share/revoke parity");
}

const billingWebhooksCovered = content
	.split("\n")
	.some((line) => line.startsWith("| Billing/Usage | webhooks |") && /\| covered \|$/.test(line));
if (billingWebhooksCovered) {
	requirePassedEvidence(billingWebhookEvidencePath, "covered billing webhook parity");
}

const billingEntitlementsCovered = content
	.split("\n")
	.some((line) => line.startsWith("| Billing/Usage | entitlements |") && /\| covered \|$/.test(line));
if (billingEntitlementsCovered) {
	requirePassedEvidence(billingEntitlementsEvidencePath, "covered billing entitlements parity");
}

const developerOauthTokensCovered = content
	.split("\n")
	.some((line) => line.startsWith("| Developer Platform | OAuth apps/tokens |") && /\| covered \|$/.test(line));
if (developerOauthTokensCovered) {
	requirePassedEvidence(developerOauthTokenEvidencePath, "covered developer OAuth apps/tokens parity");
}

const developerSignedWebhooksCovered = content
	.split("\n")
	.some((line) => line.startsWith("| Developer Platform | signed webhooks |") && /\| covered \|$/.test(line));
if (developerSignedWebhooksCovered) {
	requirePassedEvidence(developerSignedWebhookEvidencePath, "covered developer signed webhooks parity");
}

const cloudProvisioningCovered = content
	.split("\n")
	.some((line) => line.startsWith("| Cloud/Internal | provisioning console |") && /\| covered \|$/.test(line));
if (cloudProvisioningCovered) {
	requirePassedEvidence(cloudProvisioningEvidencePath, "covered Cloud/Internal provisioning parity");
}

function requirePassedEvidence(evidencePath, reason) {
	if (!existsSync(evidencePath)) {
		errors.push(`${evidencePath}: missing for ${reason}`);
		return;
	}
	const evidence = JSON.parse(readFileSync(evidencePath, "utf8"));
	if (evidence.summary?.status !== "passed") {
		errors.push(`${evidencePath}: evidence must be passed for ${reason}`);
	}
}

function tableRowsAfter(heading) {
	const start = content.indexOf(heading);
	if (start === -1) return [];
	const section = content.slice(start).split(/\n## /)[0];
	return section
		.split("\n")
		.filter((line) => line.startsWith("|") && !line.includes("---") && !line.includes("Domain |"))
		.map((line) => line.split("|").slice(1, -1).map((cell) => cell.trim()));
}

function tableHeaderAfter(heading) {
	const start = content.indexOf(heading);
	if (start === -1) return [];
	const section = content.slice(start).split(/\n## /)[0];
	const header = section.split("\n").find((line) => line.startsWith("|") && !line.includes("---"));
	return header ? header.split("|").slice(1, -1).map((cell) => cell.trim()) : [];
}

function validateMatrixShape() {
	const header = tableHeaderAfter("## Matrix");
	if (header.join("|") !== requiredColumns.join("|")) {
		errors.push(`## Matrix: columns must be ${requiredColumns.join(", ")}`);
	}
	const rows = tableRowsAfter("## Matrix");
	if (rows.length !== requiredCapabilities.length) {
		errors.push(`## Matrix: expected ${requiredCapabilities.length} rows, found ${rows.length}`);
	}
	const labels = rows.map(([domain, capability]) => `${domain} | ${capability}`);
	for (const capability of requiredCapabilities) {
		if (!labels.includes(capability)) errors.push(`## Matrix: missing row ${capability}`);
	}
	for (const label of labels) {
		if (!requiredCapabilities.includes(label)) errors.push(`## Matrix: unexpected row ${label}`);
	}
	for (const row of rows) {
		if (row.length !== requiredColumns.length) {
			errors.push(`## Matrix: row ${row[0] ?? "unknown"} must have ${requiredColumns.length} columns`);
		}
	}
}

function validateParityRow(row) {
	const [domain, capability, critical, owner, evidence, status] = row;
	const label = `${domain} ${capability}`;
	if (!["yes", "no"].includes(critical)) errors.push(`${label}: unsupported critical value ${critical}`);
	if (!requiredStatuses.includes(status)) errors.push(`${label}: unsupported parity status ${status}`);
	if (status === "pending") return;
	if (isEmptyMarker(owner)) errors.push(`${label}: ${status} parity requires owner`);
	if (isEmptyMarker(evidence)) errors.push(`${label}: ${status} parity requires target evidence`);
	if (status === "accepted reject" && !/reason|impact|reject/i.test(evidence)) {
		errors.push(`${label}: accepted reject parity requires reason, impact or reject evidence`);
	}
	if (status === "removed" && !/decision|approval|removed/i.test(evidence)) {
		errors.push(`${label}: removed parity requires product decision evidence`);
	}
}

function isEmptyMarker(value) {
	return ["", "pending", "none", "unassigned", "no-go"].includes(value ?? "");
}

function validateStatusSummary() {
	const rows = tableRowsAfter("## Matrix");
	const expected = {
		total_capabilities: rows.length,
		covered_capabilities: rows.filter((row) => row[5] === "covered").length,
		pending_capabilities: rows.filter((row) => row[5] === "pending").length,
		critical_capabilities: rows.filter((row) => row[2] === "yes").length,
	};
	for (const [key, value] of Object.entries(expected)) {
		const match = content.match(new RegExp(`- ${key}: (\\d+)`));
		if (!match) {
			errors.push(`## Status: missing ${key}`);
		} else if (Number(match[1]) !== value) {
			errors.push(`## Status: ${key} must be ${value}, found ${match[1]}`);
		}
	}
}

if (errors.length > 0) {
	console.error("Parity checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(`Parity: ok (${requiredCapabilities.length} capabilities)`);
