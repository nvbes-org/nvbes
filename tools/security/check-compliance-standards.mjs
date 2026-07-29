#!/usr/bin/env node
import { existsSync, readFileSync } from "node:fs";

const registryPath = "docs/compliance/standards-control-matrix.json";
const expectedFrameworks = new Set([
	"OWASP-ASVS-5.0.0",
	"NIST-SP-800-63-4",
	"OAUTH-OIDC-FAPI",
	"NIST-CSF-2.0-CIS-8.1",
	"ISO-27001-27017-27018",
	"GDPR-CNIL",
	"PCI-DSS-4.0.1",
	"SOC2-TYPE-II",
	"NIS2-CRA",
]);
const externalFrameworks = new Set([
	"ISO-27001-27017-27018",
	"PCI-DSS-4.0.1",
	"SOC2-TYPE-II",
	"NIS2-CRA",
]);
const validStatuses = new Set([
	"implemented",
	"partial",
	"planned",
	"not-applicable",
]);
const forbiddenClaims = new Set([
	"certified",
	"externally attested",
	"fully compliant",
]);
const errors = [];

function array(value, context) {
	if (Array.isArray(value)) return value;
	errors.push(`${context}: must be an array`);
	return [];
}

function string(value, context) {
	if (typeof value === "string" && value.trim()) return value;
	errors.push(`${context}: must be a non-empty string`);
	return "";
}

function httpsUrl(value, context) {
	const text = string(value, context);
	try {
		if (new URL(text).protocol !== "https:")
			errors.push(`${context}: must use HTTPS`);
	} catch {
		errors.push(`${context}: must be a valid URL`);
	}
	return text;
}

function uniqueId(value, context, seen, pattern) {
	const id = string(value, `${context}.id`);
	if (id && !pattern.test(id)) errors.push(`${context}.id: invalid ID ${id}`);
	if (seen.has(id)) errors.push(`${context}.id: duplicate ID ${id}`);
	seen.add(id);
	return id;
}

function verifyEvidence(entry, context) {
	const path = string(entry?.path, `${context}.path`);
	if (!existsSync(path)) {
		errors.push(`${context}.path: ${path} is missing`);
		return 0;
	}

	const content = readFileSync(path, "utf8");
	let count = 0;
	for (const needleValue of array(entry?.includes, `${context}.includes`)) {
		const needle = string(needleValue, `${context}.includes[]`);
		if (needle && !content.includes(needle)) {
			errors.push(
				`${context}: ${path} does not include ${JSON.stringify(needle)}`,
			);
		}
		count += 1;
	}
	return count;
}

if (!existsSync(registryPath)) {
	errors.push(`${registryPath}: missing`);
} else {
	let registry;
	try {
		registry = JSON.parse(readFileSync(registryPath, "utf8"));
	} catch (error) {
		errors.push(`${registryPath}: invalid JSON (${error.message})`);
	}

	if (registry) {
		if (registry.schemaVersion !== 1)
			errors.push(`${registryPath}: schemaVersion must be 1`);
		string(registry.owner, "owner");
		string(registry.reviewCadence, "reviewCadence");

		const claims = registry.communicationsPolicy ?? {};
		string(claims.allowedClaim, "communicationsPolicy.allowedClaim");
		const prohibited = new Set(
			array(claims.prohibitedClaims, "communicationsPolicy.prohibitedClaims"),
		);
		for (const claim of forbiddenClaims) {
			if (!prohibited.has(claim)) {
				errors.push(`communicationsPolicy.prohibitedClaims: missing ${claim}`);
			}
		}

		const profiles = array(registry.profiles, "profiles");
		const profileIds = new Set();
		const profileById = new Map();
		profiles.forEach((profile, index) => {
			const context = `profiles[${index}]`;
			const id = uniqueId(profile?.id, context, profileIds, /^NVBES_[A-Z]+$/u);
			profileById.set(id, profile);
			for (const item of array(profile?.scope, `${context}.scope`)) {
				string(item, `${context}.scope[]`);
			}
		});

		const baseline = profileById.get("NVBES_BASELINE");
		const critical = profileById.get("NVBES_CRITICAL");
		if (baseline?.asvsLevel !== 2)
			errors.push("NVBES_BASELINE: asvsLevel must be 2");
		if (critical?.asvsLevel !== 3)
			errors.push("NVBES_CRITICAL: asvsLevel must be 3");
		if (critical?.aal !== "AAL2")
			errors.push("NVBES_CRITICAL: aal must be AAL2");
		if (critical?.phishingResistant !== true) {
			errors.push("NVBES_CRITICAL: phishingResistant must be true");
		}
		for (const scope of [
			"Account",
			"Backoffice",
			"Billing",
			"privileged operations",
		]) {
			if (!critical?.scope?.includes(scope))
				errors.push(`NVBES_CRITICAL.scope: missing ${scope}`);
		}

		const frameworks = array(registry.frameworks, "frameworks");
		const frameworkIds = new Set();
		const frameworkCoverage = new Map();
		frameworks.forEach((framework, index) => {
			const context = `frameworks[${index}]`;
			const id = uniqueId(
				framework?.id,
				context,
				frameworkIds,
				/^[A-Z0-9][A-Z0-9.-]+$/u,
			);
			httpsUrl(framework?.source, `${context}.source`);
			for (const source of array(
				framework?.relatedSources ?? [],
				`${context}.relatedSources`,
			)) {
				httpsUrl(source, `${context}.relatedSources[]`);
			}
			string(framework?.target, `${context}.target`);
			string(framework?.claimStatus, `${context}.claimStatus`);
			frameworkCoverage.set(id, 0);

			if (
				externalFrameworks.has(id) &&
				framework?.externalAssuranceRequired !== true
			) {
				errors.push(`${context}: externalAssuranceRequired must be true`);
			}
			if (
				externalFrameworks.has(id) &&
				forbiddenClaims.has(framework?.claimStatus)
			) {
				errors.push(
					`${context}: unsupported assurance claim ${framework.claimStatus}`,
				);
			}
		});
		for (const id of expectedFrameworks) {
			if (!frameworkIds.has(id)) errors.push(`frameworks: missing ${id}`);
		}

		const controls = array(registry.controls, "controls");
		const controlIds = new Set();
		let evidenceCount = 0;
		controls.forEach((control, index) => {
			const context = `controls[${index}]`;
			uniqueId(control?.id, context, controlIds, /^STD_CTRL_\d{3}$/u);
			string(control?.name, `${context}.name`);

			if (!validStatuses.has(control?.status)) {
				errors.push(`${context}.status: invalid status ${control?.status}`);
			}
			if (control?.status !== "implemented") {
				string(control?.gap, `${context}.gap`);
				string(control?.target, `${context}.target`);
			}
			for (const profileId of array(control?.profiles, `${context}.profiles`)) {
				if (!profileIds.has(profileId))
					errors.push(`${context}.profiles: unknown ${profileId}`);
			}
			for (const frameworkId of array(
				control?.frameworks,
				`${context}.frameworks`,
			)) {
				if (!frameworkIds.has(frameworkId)) {
					errors.push(`${context}.frameworks: unknown ${frameworkId}`);
				} else {
					frameworkCoverage.set(
						frameworkId,
						frameworkCoverage.get(frameworkId) + 1,
					);
				}
			}

			const entries = array(control?.evidence, `${context}.evidence`);
			if (entries.length === 0)
				errors.push(`${context}.evidence: must not be empty`);
			entries.forEach((entry, evidenceIndex) => {
				evidenceCount += verifyEvidence(
					entry,
					`${context}.evidence[${evidenceIndex}]`,
				);
			});
		});
		for (const [frameworkId, count] of frameworkCoverage) {
			if (count === 0)
				errors.push(`frameworks: ${frameworkId} has no mapped control`);
		}

		if (errors.length === 0) {
			const partialCount = controls.filter(
				(control) => control.status === "partial",
			).length;
			console.log(
				`Compliance standards: ok (${profiles.length} profiles, ${frameworks.length} frameworks, ${controls.length} controls, ${evidenceCount} evidence strings, ${partialCount} documented gaps)`,
			);
		}
	}
}

if (errors.length > 0) {
	console.error("Compliance standards failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}
