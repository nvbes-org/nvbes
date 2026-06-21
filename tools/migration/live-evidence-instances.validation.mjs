import { requirements as packetRequirements, strictCommandFor } from "./cutover-evidence-packet.requirements.mjs";
import { buildPreparationCommands } from "./live-evidence-commands.mjs";
import { requiredEvidenceFor } from "./live-evidence.rules.mjs";

export function validateManifest(manifest, { jsonPath, strict }) {
	const errors = [];
	if (manifest.schema_version !== 1) errors.push(`${jsonPath}: schema_version must be 1`);
	if (manifest.generation?.command !== "tools/migration/live-evidence-instances.mjs --write") {
		errors.push(`${jsonPath}: generation.command is invalid`);
	}
	if (manifest.generation?.strict_command !== "tools/migration/live-evidence-instances.mjs --strict") {
		errors.push(`${jsonPath}: generation.strict_command is invalid`);
	}
	if (!Array.isArray(manifest.requirements) || manifest.requirements.length === 0) {
		errors.push(`${jsonPath}: requirements must be present`);
	}
	if (!manifest.status || typeof manifest.status !== "object") {
		errors.push(`${jsonPath}: status must be present`);
	}
	validateRequirementSet(manifest, jsonPath, errors);
	validateStatusConsistency(manifest, jsonPath, errors);
	for (const requirement of manifest.requirements ?? []) {
		validateRequirement(requirement, errors);
	}
	if (strict && manifest.status?.decision !== "go") {
		errors.push(`${jsonPath}: live evidence instances decision is ${manifest.status?.decision}`);
	}
	return errors;
}

function validateStatusConsistency(manifest, jsonPath, errors) {
	const requirements = manifest.requirements ?? [];
	const status = manifest.status ?? {};
	const instanceCount = requirements.reduce((sum, requirement) => {
		const instances = Array.isArray(requirement.instances) ? requirement.instances : [];
		return sum + instances.length;
	}, 0);
	const missingRequirements = requirements.filter((requirement) => !requirement.ready).length;
	const missingEvidenceItems = requirements.reduce((sum, requirement) => sum + (requirement.missing_evidence?.length ?? 0), 0);
	for (const field of ["instance_count", "requirements", "missing_requirements", "missing_evidence_items"]) {
		if (!Number.isInteger(status[field]) || status[field] < 0) {
			errors.push(`${jsonPath}: status.${field} must be a non-negative integer`);
		}
	}
	if (!["go", "no-go"].includes(status.decision)) {
		errors.push(`${jsonPath}: status.decision must be go or no-go`);
	}
	if (status.instance_count !== instanceCount) {
		errors.push(`${jsonPath}: instance_count must match attached instance rows`);
	}
	if (status.requirements !== requirements.length) {
		errors.push(`${jsonPath}: requirements count must match requirement rows`);
	}
	if (status.missing_requirements !== missingRequirements) {
		errors.push(`${jsonPath}: missing_requirements must match not-ready rows`);
	}
	if (status.missing_evidence_items !== missingEvidenceItems) {
		errors.push(`${jsonPath}: missing_evidence_items must match missing evidence rows`);
	}
	if (status.decision === "go" && missingRequirements !== 0) {
		errors.push(`${jsonPath}: go decision requires zero missing requirements`);
	}
	if (status.decision === "no-go" && missingRequirements === 0) {
		errors.push(`${jsonPath}: no-go decision requires at least one missing requirement`);
	}
}

function validateRequirementSet(manifest, jsonPath, errors) {
	const ids = new Set();
	const expectedById = new Map(packetRequirements.map((requirement) => [requirement.id, requirement]));
	for (const requirement of manifest.requirements ?? []) {
		if (!requirement.id) {
			errors.push(`${jsonPath}: requirement id is required`);
			continue;
		}
		if (ids.has(requirement.id)) errors.push(`${jsonPath}: duplicate requirement ${requirement.id}`);
		ids.add(requirement.id);
		const expected = expectedById.get(requirement.id);
		if (!expected) {
			errors.push(`${jsonPath}: unexpected requirement ${requirement.id}`);
			continue;
		}
		if (requirement.scope !== expected.scope) errors.push(`${requirement.id}: scope must match cutover packet requirement`);
		if (requirement.strict_command !== strictCommandFor(expected)) {
			errors.push(`${requirement.id}: strict_command must match cutover packet requirement`);
		}
		const expectedEvidence = requiredEvidenceFor(requirement.id);
		if (!sameJson(requirement.expected_evidence, expectedEvidence)) {
			errors.push(`${requirement.id}: expected_evidence must match live evidence rules`);
		}
		const commands = buildPreparationCommands([{ ...requirement, expected_evidence: expectedEvidence }]);
		if (!sameJson(requirement.preparation_commands, commands)) {
			errors.push(`${requirement.id}: preparation_commands must match live evidence rules`);
		}
	}
	for (const requirement of packetRequirements) {
		if (!ids.has(requirement.id)) errors.push(`${jsonPath}: missing requirement ${requirement.id}`);
	}
}

function validateRequirement(requirement, errors) {
	if (!Array.isArray(requirement.expected_evidence_types) || requirement.expected_evidence_types.length === 0) {
		errors.push(`${requirement.id}: expected_evidence_types are required`);
	}
	if (!sameJson(requirement.expected_evidence_types, evidenceTypesFor(requirement.expected_evidence ?? []))) {
		errors.push(`${requirement.id}: expected_evidence_types must match expected_evidence`);
	}
	if (!Array.isArray(requirement.missing_evidence_types)) {
		errors.push(`${requirement.id}: missing_evidence_types are required`);
	}
	if (!Array.isArray(requirement.accepted_evidence_types)) {
		errors.push(`${requirement.id}: accepted_evidence_types are required`);
	}
	if (!sameJson(requirement.accepted_evidence_types, acceptedTypesFor(requirement.instances ?? []))) {
		errors.push(`${requirement.id}: accepted_evidence_types must match accepted instances`);
	}
	if (!Array.isArray(requirement.missing_evidence)) {
		errors.push(`${requirement.id}: missing_evidence is required`);
	}
	if (!Array.isArray(requirement.preparation_commands)) {
		errors.push(`${requirement.id}: preparation_commands are required`);
	} else if (!requirement.ready && requirement.preparation_commands.length === 0) {
		errors.push(`${requirement.id}: missing requirement must include preparation_commands`);
	}
	if (!sameJson(requirement.missing_evidence_types, missingTypesFor(requirement.missing_evidence ?? []))) {
		errors.push(`${requirement.id}: missing_evidence_types must match missing_evidence`);
	}
	if (!Array.isArray(requirement.instances)) {
		errors.push(`${requirement.id}: instances are required`);
	}
	if (requirement.ready !== (requirement.missing_evidence?.length === 0)) {
		errors.push(`${requirement.id}: ready must match missing_evidence`);
	}
	if (!requirement.strict_command) errors.push(`${requirement.id}: strict_command is required`);
	for (const instance of requirement.instances ?? []) {
		validateInstanceSummary(requirement, instance, errors);
	}
}

function validateInstanceSummary(requirement, instance, errors) {
	if (!Array.isArray(instance.checksum_algorithms)) {
		errors.push(`${requirement.id}: instance checksum_algorithms are required`);
	}
	if (!instance.source_artifact) errors.push(`${requirement.id}: instance source_artifact is required`);
	if (!instance.immutable_reference) errors.push(`${requirement.id}: instance immutable_reference is required`);
}

function evidenceTypesFor(expectedEvidence) {
	return expectedEvidence.map((entry) => entry.type);
}

function missingTypesFor(missingEvidence) {
	return missingEvidence.map((entry) => entry.split(":")[0].split("@")[0]);
}

function acceptedTypesFor(instances) {
	return [...new Set(instances.map((instance) => instance.evidence_type).filter(Boolean))].sort();
}

function sameJson(actual, expected) {
	return JSON.stringify(actual) === JSON.stringify(expected);
}
