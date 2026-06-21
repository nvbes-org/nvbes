import { createHash } from "node:crypto";
import { existsSync, readFileSync } from "node:fs";
import { basename, isAbsolute, normalize } from "node:path";
import {
	commandReportPath,
	commandMatchesStrictCommand,
	environmentMatchesCommandArgument,
	sourceArtifactMatchesReportArgument,
} from "./live-evidence.command.mjs";
import { validateReconciliationSourceArtifact } from "./live-evidence.reconciliation.mjs";

const requirementEvidence = {
	"g4-frontend-signoff": [
		{ type: "frontend_signoff", min: 1 },
		{ type: "web_check", min: 1 },
		{ type: "smoke_test", min: 1 },
	],
	"g5-infra-signoff": [
		{ type: "infra_deploy", min: 1, environments: ["staging"] },
		{ type: "infra_restore", min: 1, environments: ["staging"] },
		{ type: "rollback", min: 1, environments: ["staging"] },
	],
	"p11-rehearsals": [{ type: "rehearsal", min: 3, environments: ["local", "staging", "production"] }, { type: "reconciliation", min: 2 }, { type: "rollback", min: 1 }, { type: "reject_review", min: 1 }],
	"p12-cutover": [
		{ type: "cutover", min: 1, environments: ["production"] },
		{ type: "reconciliation", min: 1, environments: ["production"] },
	],
	"p13-decommission": [{ type: "decommission", min: 1, environments: ["production"] }],
	"final-reconciliation": [{ type: "reconciliation", min: 1, environments: ["production"] }],
};

const maxChecksums = 4;
const checksumLengths = { sha256: 64, sha512: 128 };
const cutoverEvidencePacketPath = "docs/migration/cutover-evidence-packet.generated.json";
const maxCapturedAtClockSkewMs = 5 * 60 * 1000;
const checksumHexPattern = /^[a-fA-F0-9]+$/;
const placeholderPatterns = [/template-live-evidence/, /migration lead required/, /replace-with-/, /<run>/, /^0+$/];
const templateEvidencePatterns = [/\.template\./, /\/template[./-]/i, /^template[./-]/i];
const externalReferencePattern = /^[a-z][a-z0-9+.-]*:/i;
const immutableReferencePattern = /^(ci|artifact|s3|gs|oci):\/\//i;
const allowedLocalArtifactPrefixes = ["docs/migration/", "tools/migration/fixtures/"];

export function requiredEvidenceFor(requirementId) {
	return requirementEvidence[requirementId] ?? [];
}

export function requiredTypesFor(requirementId) {
	return requiredEvidenceFor(requirementId).map((entry) => entry.type);
}

export function buildLiveEvidenceTemplate() {
	return {
		schema_version: 1,
		evidence_id: "template-live-evidence",
		evidence_type: "reconciliation",
		environment: "staging",
		captured_at: "1970-01-01T00:00:00.000Z",
		owner: "migration lead required",
		source_artifact: "docs/migration/reconciliation.<run>.json",
		command: "node tools/migration/reconcile.mjs --env staging --report docs/migration/reconciliation.<run>.json",
		result: "blocking",
		decision: "no-go",
		immutable_reference: "replace-with-ci-or-artifact-uri",
		packet_requirement: "final-reconciliation",
		checksums: [
			{ name: "source_artifact", algorithm: "sha256", value: "0".repeat(64) },
			{ name: "cutover_evidence_packet", algorithm: "sha256", value: "0".repeat(64) },
		],
		notes: "Template only. Prefer tools/migration/live-evidence-prepare.mjs after a real run. For local source_artifact paths, the checksum value must match the file digest. The cutover_evidence_packet checksum must match docs/migration/cutover-evidence-packet.generated.json.",
	};
}

export function validateLiveEvidenceInstance(instance, schema, path, { allowTemplate = false } = {}) {
	const errors = [];
	const required = schema?.required ?? [];
	const properties = schema?.properties ?? {};
	for (const field of required) {
		if (!(field in instance)) errors.push(`${path}: missing required field ${field}`);
	}
	for (const field of Object.keys(instance)) {
		if (!properties[field]) errors.push(`${path}: unsupported field ${field}`);
	}
	for (const [field, definition] of Object.entries(properties)) {
		const value = instance[field];
		if (value === undefined) continue;
		if (definition.type === "string" && typeof value !== "string") errors.push(`${path}: ${field} must be a string`);
		if (definition.type === "integer" && (!Number.isInteger(value) || typeof value !== "number")) {
			errors.push(`${path}: ${field} must be an integer`);
		}
		if (definition.type === "string" && definition.minLength && value.length < definition.minLength) {
			errors.push(`${path}: ${field} must be at least ${definition.minLength} chars`);
		}
		if (definition.type === "string" && definition.pattern && !new RegExp(definition.pattern).test(value)) {
			errors.push(`${path}: ${field} must match ${definition.pattern}`);
		}
		if (definition.enum && !definition.enum.includes(value)) errors.push(`${path}: ${field} must be one of ${definition.enum.join(", ")}`);
		if (definition.const !== undefined && value !== definition.const) errors.push(`${path}: ${field} must be ${definition.const}`);
	}
	const capturedAtMs = Date.parse(instance.captured_at ?? "");
	if (Number.isNaN(capturedAtMs)) {
		errors.push(`${path}: captured_at must be an ISO date-time`);
	} else if (!allowTemplate && capturedAtMs > Date.now() + maxCapturedAtClockSkewMs) {
		errors.push(`${path}: captured_at must not be in the future`);
	}
	if (instance.decision === "go" && !["passed", "accepted"].includes(instance.result)) {
		errors.push(`${path}: decision go requires result passed or accepted`);
	}
	if (instance.decision === "go" && !immutableReferencePattern.test(instance.immutable_reference ?? "")) {
		errors.push(`${path}: decision go requires immutable_reference to use an approved immutable URI scheme`);
	}
	if (instance.decision === "go" && !sourceArtifactReferenceIsApproved(instance.source_artifact)) {
		errors.push(`${path}: decision go requires source_artifact to be local approved evidence or use an approved immutable URI scheme`);
	}
	if (instance.decision === "go" && !hasSourceArtifactChecksum(instance.checksums)) {
		errors.push(`${path}: decision go requires a checksum named source_artifact`);
	}
	if (instance.decision === "go" && !hasNamedChecksum(instance.checksums, "cutover_evidence_packet")) {
		errors.push(`${path}: decision go requires a checksum named cutover_evidence_packet`);
	}
	if (instance.decision === "go" && usesTemplateEvidence(instance)) {
		errors.push(`${path}: decision go cannot use template artifacts`);
	}
	if (instance.decision === "go" && !sourceArtifactMatchesReportArgument(instance)) {
		errors.push(`${path}: source_artifact must match command --report path`);
	}
	if (instance.decision === "go" && !localSourceArtifactIsApproved(instance.source_artifact)) {
		errors.push(`${path}: local source_artifact must stay under approved migration evidence paths`);
	}
	if (instance.decision === "go" && !environmentMatchesCommandArgument(instance)) {
		errors.push(`${path}: environment must match command --env value`);
	}
	if (instance.result === "accepted" && (typeof instance.notes !== "string" || instance.notes.trim().length < 12)) {
		errors.push(`${path}: accepted result requires notes with owner justification`);
	}
	if (["failed", "blocking"].includes(instance.result) && instance.decision !== "no-go") {
		errors.push(`${path}: failed or blocking result requires decision no-go`);
	}
	if (!Array.isArray(instance.checksums) || instance.checksums.length === 0) errors.push(`${path}: checksums must contain at least one checksum`);
	if ((instance.checksums ?? []).length > maxChecksums) errors.push(`${path}: checksums must contain at most ${maxChecksums} entries`);
	const checksumNames = new Set();
	for (const checksum of instance.checksums ?? []) {
		for (const field of ["name", "algorithm", "value"]) {
			if (!checksum[field]) errors.push(`${path}: checksum ${field} is required`);
		}
		if (checksum.name) {
			if (checksumNames.has(checksum.name)) errors.push(`${path}: duplicate checksum name ${checksum.name}`);
			checksumNames.add(checksum.name);
		}
		const expectedLength = checksumLengths[checksum.algorithm];
		if (!expectedLength) errors.push(`${path}: checksum algorithm must be sha256 or sha512`);
		if (expectedLength && checksum.value?.length !== expectedLength) {
			errors.push(`${path}: ${checksum.algorithm} checksum must be ${expectedLength} chars`);
		}
		if (typeof checksum.value === "string" && !checksumHexPattern.test(checksum.value)) {
			errors.push(`${path}: checksum value must be hexadecimal`);
		}
	}
	errors.push(...validateLocalArtifactChecksums(instance, path, { allowTemplate }));
	errors.push(...validateCutoverPacketChecksum(instance, path));
	const expectedTypes = requiredTypesFor(instance.packet_requirement);
	if (instance.packet_requirement && expectedTypes.length === 0) errors.push(`${path}: unknown packet_requirement ${instance.packet_requirement}`);
	if (expectedTypes.length > 0 && !expectedTypes.includes(instance.evidence_type)) {
		errors.push(`${path}: evidence_type ${instance.evidence_type} is not valid for ${instance.packet_requirement}; expected ${expectedTypes.join(", ")}`);
	}
	if (!allowTemplate) {
		const expectedEvidence = requiredEvidenceFor(instance.packet_requirement).find((entry) => entry.type === instance.evidence_type);
		if (expectedEvidence?.environments && !expectedEvidence.environments.includes(instance.environment)) {
			errors.push(`${path}: environment ${instance.environment} is not valid for ${instance.packet_requirement}/${instance.evidence_type}; expected ${expectedEvidence.environments.join(", ")}`);
		}
	}
	if (!allowTemplate) {
		const serializedValues = [
			instance.evidence_id,
			instance.owner,
			instance.source_artifact,
			instance.command,
			instance.immutable_reference,
			...(instance.checksums ?? []).flatMap((checksum) => [checksum.name, checksum.value]),
		].filter(Boolean);
		for (const value of serializedValues) {
			if (placeholderPatterns.some((pattern) => pattern.test(value))) errors.push(`${path}: placeholder value remains: ${value}`);
		}
	}
	return errors;
}

export function validateLiveEvidenceInstanceSet(records, packet) {
	const errors = [];
	const acceptedReferences = new Map();
	const acceptedSourceArtifacts = new Map();
	const requirements = new Map((packet?.requirements ?? []).map((requirement) => [requirement.id, requirement]));
	for (const { path, instance } of records) {
		if (instance?.evidence_id && instance.evidence_id !== evidenceIdForPath(path)) {
			errors.push(`${path}: evidence_id must match instance filename`);
		}
		if (!instance?.immutable_reference) continue;
		if (instance.decision !== "go" || !["passed", "accepted"].includes(instance.result)) continue;
		const existingPath = acceptedReferences.get(instance.immutable_reference);
		if (existingPath) {
			errors.push(`${path}: duplicate accepted immutable_reference ${instance.immutable_reference} already used by ${existingPath}`);
		}
		acceptedReferences.set(instance.immutable_reference, path);
		const sourceArtifactKey = normalizedArtifactKey(instance.source_artifact);
		const existingArtifactPath = acceptedSourceArtifacts.get(sourceArtifactKey);
		if (existingArtifactPath) {
			errors.push(`${path}: duplicate accepted source_artifact ${instance.source_artifact} already used by ${existingArtifactPath}`);
		}
		acceptedSourceArtifacts.set(sourceArtifactKey, path);
		const requirement = requirements.get(instance.packet_requirement);
		if (requirement?.strict_command && !commandMatchesStrictCommand(instance.command, requirement.strict_command)) {
			errors.push(`${path}: command must match strict_command for ${instance.packet_requirement}`);
		}
		if (requirement && !sourceArtifactBelongsToPacket(instance, requirement)) {
			errors.push(`${path}: local source_artifact must be referenced by the packet requirement`);
		}
	}
	return errors;
}

function evidenceIdForPath(path) {
	return basename(path, ".json");
}

function validateLocalArtifactChecksums(instance, path, { allowTemplate }) {
	if (allowTemplate) return [];
	const artifactPath = localPathFor(instance.source_artifact);
	if (!artifactPath) return [];
	if (!existsSync(artifactPath)) {
		if (instance.decision === "go") return [`${path}: local source_artifact does not exist: ${instance.source_artifact}`];
		return [];
	}
	const sourceChecksum = (instance.checksums ?? []).find((checksum) => checksum.name === "source_artifact");
	const digestMatch =
		sourceChecksum && checksumLengths[sourceChecksum.algorithm]
			? sourceChecksum.value === digestFor(artifactPath, sourceChecksum.algorithm)
			: false;
	const errors = digestMatch ? [] : [`${path}: checksum named source_artifact must match ${instance.source_artifact}`];
	errors.push(...validateReconciliationSourceArtifact(instance, path, artifactPath));
	return errors;
}

function hasSourceArtifactChecksum(checksums) {
	return hasNamedChecksum(checksums, "source_artifact");
}

function hasNamedChecksum(checksums, name) {
	return (checksums ?? []).some((checksum) => checksum.name === name && checksumLengths[checksum.algorithm]);
}

function validateCutoverPacketChecksum(instance, path) {
	if (instance.decision !== "go") return [];
	const packetChecksum = (instance.checksums ?? []).find((checksum) => checksum.name === "cutover_evidence_packet");
	if (!packetChecksum || !checksumLengths[packetChecksum.algorithm]) return [];
	if (!existsSync(cutoverEvidencePacketPath)) return [`${path}: cutover evidence packet is missing`];
	const expected = digestFor(cutoverEvidencePacketPath, packetChecksum.algorithm);
	return packetChecksum.value === expected
		? []
		: [`${path}: cutover_evidence_packet checksum must match ${cutoverEvidencePacketPath}`];
}

function usesTemplateEvidence(instance) {
	return [instance.source_artifact, instance.command].filter(Boolean).some((value) =>
		templateEvidencePatterns.some((pattern) => pattern.test(value)),
	);
}

function localPathFor(value) {
	if (typeof value !== "string" || value.length === 0) return undefined;
	if (externalReferencePattern.test(value) || isAbsolute(value)) return undefined;
	const normalized = normalize(value);
	if (normalized.startsWith("..")) return undefined;
	return normalized;
}

function localSourceArtifactIsApproved(value) {
	const localPath = localPathFor(value);
	return !localPath || allowedLocalArtifactPrefixes.some((prefix) => localPath.startsWith(prefix));
}

function sourceArtifactReferenceIsApproved(value) {
	return !externalReferencePattern.test(value ?? "") || immutableReferencePattern.test(value ?? "");
}

function sourceArtifactBelongsToPacket(instance, requirement) {
	const sourceArtifact = localPathFor(instance.source_artifact);
	if (!sourceArtifact) return true;
	if (commandReportPath(instance.command)) return true;
	return (requirement.files ?? []).map((file) => normalize(file)).includes(sourceArtifact);
}

function normalizedArtifactKey(value) {
	return localPathFor(value) ?? String(value ?? "").trim();
}

function digestFor(path, algorithm) {
	return createHash(algorithm).update(readFileSync(path)).digest("hex");
}
