#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";

const args = process.argv.slice(2);
const write = args.includes("--write");
const strict = args.includes("--strict");
const markdownPath = "docs/migration/live-evidence.md";
const schemaPath = "docs/migration/live-evidence.schema.json";
const packetPath = "docs/migration/cutover-evidence-packet.generated.json";
const errors = [];

const evidenceTypes = [
	"frontend_signoff",
	"web_check",
	"smoke_test",
	"infra_restore",
	"infra_deploy",
	"rehearsal",
	"rollback",
	"reject_review",
	"cutover",
	"reconciliation",
	"decommission",
];

const requiredFields = [
	"schema_version",
	"evidence_id",
	"evidence_type",
	"environment",
	"captured_at",
	"owner",
	"source_artifact",
	"command",
	"result",
	"decision",
	"immutable_reference",
	"packet_requirement",
	"checksums",
];
const evidenceIdPattern = "^[a-z0-9][a-z0-9.-]{6,126}[a-z0-9]$";
const maxChecksums = 4;

function readJson(path) {
	if (!existsSync(path)) {
		errors.push(`${path}: missing`);
		return undefined;
	}
	try {
		return JSON.parse(readFileSync(path, "utf8"));
	} catch (error) {
		errors.push(`${path}: invalid JSON: ${error.message}`);
		return undefined;
	}
}

function packetRequirementIds() {
	const packet = readJson(packetPath);
	return packet?.requirements?.map((row) => row.id) ?? [];
}

function buildSchema(packetRequirements) {
	return {
		$schema: "https://json-schema.org/draft/2020-12/schema",
		$id: "https://nvbes.local/migration/live-evidence.schema.json",
		title: "nvbes live migration evidence",
		type: "object",
		additionalProperties: false,
		required: requiredFields,
		properties: {
				schema_version: { type: "integer", const: 1 },
				evidence_id: { type: "string", minLength: 8, pattern: evidenceIdPattern },
			evidence_type: { type: "string", enum: evidenceTypes },
			environment: { type: "string", enum: ["local", "staging", "production"] },
			captured_at: { type: "string", format: "date-time" },
			owner: { type: "string", minLength: 3 },
			source_artifact: { type: "string", minLength: 3 },
			command: { type: "string", minLength: 3 },
			result: { type: "string", enum: ["passed", "accepted", "failed", "blocking"] },
			decision: { type: "string", enum: ["go", "no-go"] },
			immutable_reference: { type: "string", minLength: 8 },
			packet_requirement: { type: "string", enum: packetRequirements },
			checksums: {
				type: "array",
				minItems: 1,
				maxItems: maxChecksums,
				items: {
					type: "object",
					additionalProperties: false,
					required: ["name", "algorithm", "value"],
					properties: {
						name: { type: "string", minLength: 1 },
						algorithm: { type: "string", enum: ["sha256", "sha512"] },
						value: { type: "string", minLength: 32, pattern: "^[a-fA-F0-9]+$" },
					},
				},
			},
			notes: { type: "string" },
		},
	};
}

function buildMarkdown(schema, packetRequirements) {
	const lines = [
		"# Live Evidence Schema",
		"",
		"## Status",
		"",
		`- evidence_types: ${evidenceTypes.length}`,
		`- required_fields: ${requiredFields.length}`,
		`- cutover_packet_requirements: ${packetRequirements.length}`,
		`- checksum_max_items: ${maxChecksums}`,
		"- decision: no-go until production evidence instances validate against the schema",
		"",
		"## Evidence Types",
		"",
		"| Type | Purpose |",
		"|---|---|",
		"| frontend_signoff | G4 E2E and accessibility evidence |",
		"| web_check | G4 full web format, lint and typecheck evidence |",
		"| smoke_test | G4/G7 strict smoke-test evidence |",
		"| infra_restore | G5 staging rebuild, backup and restore evidence |",
		"| infra_deploy | G5 infrastructure deployment evidence |",
		"| rehearsal | P11/G6 migration rehearsal evidence |",
		"| rollback | rollback duration and recovery proof |",
		"| reject_review | P11/G6 reject log review evidence |",
		"| cutover | P12/G7 production cutover journal evidence |",
		"| reconciliation | production or rehearsal reconciliation evidence |",
		"| decommission | P13/G8 legacy shutdown evidence |",
		"",
		"## Required Fields",
		"",
		"| Field | Required |",
		"|---|---:|",
	];
	for (const field of schema.required) lines.push(`| ${field} | yes |`);
	lines.push(
		"",
		"## Packet Requirements",
		"",
		"| Requirement | Evidence Instance Required |",
		"|---|---:|",
	);
	for (const requirement of packetRequirements) lines.push(`| ${requirement} | yes |`);
	lines.push(
		"",
		"## Decision",
		"",
		"The schema is ready. Cutover remains no-go until each required live evidence instance exists, is immutable, and validates against this contract.",
		"",
		"## Verification",
		"",
		"```bash",
		"pnpm check:migration-live-evidence-schema",
		"pnpm check:migration-live-evidence-rules",
		"```",
		"",
	);
	return lines.join("\n");
}

function validate(schema) {
	if (schema.type !== "object") errors.push(`${schemaPath}: root type must be object`);
	if (!Array.isArray(schema.required)) errors.push(`${schemaPath}: required must be an array`);
	for (const field of requiredFields) {
		if (!schema.required?.includes(field)) errors.push(`${schemaPath}: missing required field ${field}`);
		if (!schema.properties?.[field]) errors.push(`${schemaPath}: missing property ${field}`);
	}
	const enumValues = schema.properties?.evidence_type?.enum ?? [];
	for (const type of evidenceTypes) {
		if (!enumValues.includes(type)) errors.push(`${schemaPath}: missing evidence type ${type}`);
	}
	const packetRequirementEnum = schema.properties?.packet_requirement?.enum ?? [];
	if (schema.properties?.checksums?.maxItems !== maxChecksums) errors.push(`${schemaPath}: checksums maxItems must be ${maxChecksums}`);
	const packetRequirements = packetRequirementIds();
	for (const requirement of packetRequirements) {
		if (!packetRequirementEnum.includes(requirement)) errors.push(`${schemaPath}: missing packet requirement ${requirement}`);
	}
	for (const requirement of packetRequirementEnum) {
		if (!packetRequirements.includes(requirement)) errors.push(`${schemaPath}: unknown packet requirement ${requirement}`);
	}
	if (!existsSync(packetPath)) errors.push(`${packetPath}: missing`);
	if (strict) errors.push(`${schemaPath}: strict live evidence validation requires production evidence instances`);
}

const packetRequirements = packetRequirementIds();
const schema = buildSchema(packetRequirements);
const markdown = buildMarkdown(schema, packetRequirements);
const schemaJson = `${JSON.stringify(schema, null, 2)}\n`;

if (write) {
	mkdirSync(dirname(schemaPath), { recursive: true });
	writeFileSync(schemaPath, schemaJson);
	writeFileSync(markdownPath, markdown);
	console.log(`Live evidence schema written to ${markdownPath} and ${schemaPath}`);
	process.exit(0);
}

validate(schema);

for (const [path, expected] of [[schemaPath, schemaJson], [markdownPath, markdown]]) {
	if (!existsSync(path)) {
		errors.push(`${path}: missing; run tools/migration/live-evidence-schema.mjs --write`);
	} else if (readFileSync(path, "utf8") !== expected) {
		errors.push(`${path}: stale; run tools/migration/live-evidence-schema.mjs --write`);
	}
}

if (errors.length > 0) {
	console.error("Live evidence schema checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(
	`Live evidence schema: ok (${evidenceTypes.length} types, ${requiredFields.length} required fields, ${packetRequirements.length} packet requirements)`,
);
