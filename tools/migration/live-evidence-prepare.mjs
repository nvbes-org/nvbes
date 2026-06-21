#!/usr/bin/env node
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { optionValue } from "./cli-options.mjs";
import { validateLiveEvidenceInstance } from "./live-evidence.rules.mjs";

const schemaPath = "docs/migration/live-evidence.schema.json";
const packetPath = "docs/migration/cutover-evidence-packet.generated.json";
const immutableReferencePattern = /^(ci|artifact|s3|gs|oci):\/\//i;

export function buildLiveEvidenceInstance(args, errors = []) {
	const sourceArtifact = requiredOption(args, "--source-artifact", errors);
	const algorithm = optionValue(args, "--algorithm", "sha256");
	if (!["sha256", "sha512"].includes(algorithm)) {
		errors.push("--algorithm must be sha256 or sha512");
	}
	if (sourceArtifact && !isExternalReference(sourceArtifact) && !existsSync(sourceArtifact)) {
		errors.push(`${sourceArtifact}: source artifact not found`);
	}
	if (sourceArtifact && isExternalReference(sourceArtifact) && !immutableReferencePattern.test(sourceArtifact)) {
		errors.push("--source-artifact external references must use ci://, artifact://, s3://, gs:// or oci://");
	}
	if (!existsSync(packetPath)) errors.push(`${packetPath}: cutover evidence packet not found`);

	const instance = {
		schema_version: 1,
		evidence_id: requiredOption(args, "--id", errors),
		evidence_type: requiredOption(args, "--type", errors),
		environment: requiredOption(args, "--env", errors),
		captured_at: optionValue(args, "--captured-at", new Date().toISOString()),
		owner: requiredOption(args, "--owner", errors),
		source_artifact: sourceArtifact,
		command: requiredOption(args, "--command", errors),
		result: requiredOption(args, "--result", errors),
		decision: requiredOption(args, "--decision", errors),
		immutable_reference: requiredOption(args, "--immutable-reference", errors),
		packet_requirement: requiredOption(args, "--packet-requirement", errors),
		checksums: [],
	};
	const notes = optionValue(args, "--notes");
	if (notes) instance.notes = notes;

	if (sourceArtifact && !isExternalReference(sourceArtifact) && existsSync(sourceArtifact)) {
		instance.checksums.push({
			name: "source_artifact",
			algorithm,
			value: digestFor(sourceArtifact, algorithm),
		});
	} else if (sourceArtifact && isExternalReference(sourceArtifact)) {
		const sourceChecksum = optionValue(args, "--source-checksum");
		if (!sourceChecksum) {
			errors.push("--source-checksum is required when --source-artifact is an external reference");
		} else {
			instance.checksums.push({
				name: "source_artifact",
				algorithm,
				value: sourceChecksum,
			});
		}
	}
	if (existsSync(packetPath)) {
		instance.checksums.push({
			name: "cutover_evidence_packet",
			algorithm,
			value: digestFor(packetPath, algorithm),
		});
	}
	return instance;
}

export function validatePreparedLiveEvidence(instance, path = "prepared-live-evidence") {
	const schema = JSON.parse(readFileSync(schemaPath, "utf8"));
	return validateLiveEvidenceInstance(instance, schema, path);
}

function requiredOption(args, flag, errors) {
	const value = optionValue(args, flag);
	if (!value) errors.push(`${flag} is required`);
	return value;
}

function isExternalReference(value) {
	return /^[a-z][a-z0-9+.-]*:/i.test(value);
}

function digestFor(path, algorithm) {
	return createHash(algorithm).update(readFileSync(path)).digest("hex");
}

function serializeJson(value) {
	return `${JSON.stringify(value, null, 2)}\n`;
}

if (import.meta.url === `file://${process.argv[1]}`) {
	const args = process.argv.slice(2);
	if (args.includes("--help")) {
		console.log(
			"Usage: tools/migration/live-evidence-prepare.mjs --id <id> --type <type> --env <env> --owner <owner> --source-artifact <path-or-uri> --command <command> --result <passed|accepted|failed|blocking> --decision <go|no-go> --immutable-reference <uri> --packet-requirement <id> [--source-checksum <hex>] [--out <path>]",
		);
		process.exit(0);
	}
	const errors = [];
	const instance = buildLiveEvidenceInstance(args, errors);
	errors.push(...validatePreparedLiveEvidence(instance));
	if (errors.length > 0) {
		console.error("Live evidence preparation failed:");
		for (const error of errors) console.error(`- ${error}`);
		process.exit(1);
	}
	const output = serializeJson(instance);
	const outPath = optionValue(args, "--out");
	if (outPath) {
		mkdirSync(dirname(outPath), { recursive: true });
		writeFileSync(outPath, output);
		console.log(`Live evidence instance written to ${outPath}`);
	} else {
		process.stdout.write(output);
	}
}
