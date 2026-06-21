import { existsSync, readFileSync } from "node:fs";

export function validateReconciliationSourceArtifact(instance, path, artifactPath) {
	if (instance.evidence_type !== "reconciliation" || instance.decision !== "go" || !artifactPath) return [];
	if (!artifactPath.endsWith(".json")) return [];
	if (!existsSync(artifactPath)) return [];
	const errors = [];
	let report;
	try {
		report = JSON.parse(readFileSync(artifactPath, "utf8"));
	} catch (error) {
		return [`${path}: reconciliation source_artifact JSON is invalid: ${error.message}`];
	}
	if (report.schema_version !== 1) errors.push(`${path}: reconciliation report schema_version must be 1`);
	if (report.environment !== instance.environment) {
		errors.push(`${path}: reconciliation report environment must match evidence environment`);
	}
	if (!["go", "go-with-accepted-rejects"].includes(report.decision)) {
		errors.push(`${path}: reconciliation report decision must be go or go-with-accepted-rejects`);
	}
	if (!Array.isArray(report.domains) || report.domains.length === 0) {
		errors.push(`${path}: reconciliation report domains must be non-empty`);
	}
	for (const [index, domain] of (report.domains ?? []).entries()) {
		if (domain.status === "blocking") errors.push(`${path}: reconciliation report domain ${index} is blocking`);
		if (domain.rejects?.blocking > 0) errors.push(`${path}: reconciliation report domain ${index} has blocking rejects`);
		if (domain.checksums?.matched !== true && domain.status !== "accepted") {
			errors.push(`${path}: reconciliation report domain ${index} checksum mismatch requires accepted status`);
		}
	}
	return errors;
}
