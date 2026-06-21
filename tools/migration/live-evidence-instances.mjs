#!/usr/bin/env node
import { existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import {
	buildLiveEvidenceTemplate,
	requiredEvidenceFor,
	validateLiveEvidenceInstance,
	validateLiveEvidenceInstanceSet,
} from "./live-evidence.rules.mjs";
import { buildPreparationCommands } from "./live-evidence-commands.mjs";
import { validateManifest } from "./live-evidence-instances.validation.mjs";

const args = process.argv.slice(2);
const write = args.includes("--write");
const strict = args.includes("--strict");
const schemaPath = "docs/migration/live-evidence.schema.json";
const packetPath = "docs/migration/cutover-evidence-packet.generated.json";
const evidenceDir = "docs/migration/live-evidence-instances";
const templatePath = "docs/migration/live-evidence.template.json";
const jsonPath = "docs/migration/live-evidence-instances.generated.json";
const markdownPath = "docs/migration/live-evidence-instances.md";
const errors = [];

function readJson(path, required = true) {
	if (!existsSync(path)) {
		if (required) errors.push(`${path}: missing`);
		return undefined;
	}
	try {
		return JSON.parse(readFileSync(path, "utf8"));
	} catch (error) {
		errors.push(`${path}: invalid JSON: ${error.message}`);
		return undefined;
	}
}

function instanceFiles() {
	if (!existsSync(evidenceDir)) return [];
	return readdirSync(evidenceDir)
		.filter((file) => file.endsWith(".json"))
		.map((file) => join(evidenceDir, file))
		.sort();
}

function nonJsonInstanceFiles() {
	if (!existsSync(evidenceDir)) return [];
	return readdirSync(evidenceDir).filter((file) => !file.endsWith(".json")).sort();
}

function buildManifest() {
	const schema = readJson(schemaPath);
	const packet = readJson(packetPath);
	const template = buildLiveEvidenceTemplate();
	errors.push(...validateLiveEvidenceInstance(template, schema, templatePath, { allowTemplate: true }));
	const instances = instanceFiles().map((path) => {
		const instance = readJson(path);
		if (instance) errors.push(...validateLiveEvidenceInstance(instance, schema, path));
		return { path, instance };
	});
	const ids = new Set();
	for (const { path, instance } of instances) {
		if (!instance?.evidence_id) continue;
		if (ids.has(instance.evidence_id)) errors.push(`${path}: duplicate evidence_id ${instance.evidence_id}`);
		ids.add(instance.evidence_id);
	}
	errors.push(...validateLiveEvidenceInstanceSet(instances, packet));
	const requirements = (packet?.requirements ?? []).map((requirement) => {
		const matching = instances.filter(({ instance }) => instance?.packet_requirement === requirement.id);
		const accepted = matching.filter(({ instance }) =>
			instance?.decision === "go" && ["passed", "accepted"].includes(instance?.result),
		);
		const expectedEvidence = requiredEvidenceFor(requirement.id);
		const acceptedTypes = new Set(accepted.map(({ instance }) => instance?.evidence_type));
		const missingEvidence = expectedEvidence.flatMap((expected) => {
			const acceptedForType = accepted.filter(({ instance }) => instance?.evidence_type === expected.type);
			const missing = [];
			if (acceptedForType.length < expected.min) missing.push(`${expected.type}:${acceptedForType.length}/${expected.min}`);
			for (const environment of expected.environments ?? []) {
				if (!acceptedForType.some(({ instance }) => instance?.environment === environment)) {
					missing.push(`${expected.type}@${environment}`);
				}
			}
			return missing;
		});
		return {
			id: requirement.id,
			scope: requirement.scope,
			strict_command: requirement.strict_command,
			expected_evidence: expectedEvidence,
			preparation_commands: buildPreparationCommands([{ ...requirement, expected_evidence: expectedEvidence }]),
			expected_evidence_types: expectedEvidence.map((entry) => entry.type),
			accepted_evidence_types: [...acceptedTypes].sort(),
			missing_evidence: missingEvidence,
			missing_evidence_types: missingEvidence.map((entry) => entry.split(":")[0].split("@")[0]),
			instances: matching.map(({ path, instance }) => ({
				path,
				evidence_id: instance?.evidence_id,
				evidence_type: instance?.evidence_type,
				environment: instance?.environment,
				result: instance?.result,
				decision: instance?.decision,
				source_artifact: instance?.source_artifact,
				immutable_reference: instance?.immutable_reference,
				checksum_algorithms: [...new Set((instance?.checksums ?? []).map((checksum) => checksum.algorithm))].sort(),
			})),
			ready: missingEvidence.length === 0,
		};
	});
	const missing = requirements.filter((requirement) => !requirement.ready);
	const missingEvidenceItems = requirements.reduce((sum, requirement) => sum + requirement.missing_evidence.length, 0);
	return {
		schema_version: 1,
		generation: {
			command: "tools/migration/live-evidence-instances.mjs --write",
			strict_command: "tools/migration/live-evidence-instances.mjs --strict",
		},
		status: {
			instance_count: instances.length,
			requirements: requirements.length,
			missing_requirements: missing.length,
			missing_evidence_items: missingEvidenceItems,
			decision: missing.length === 0 ? "go" : "no-go",
		},
		requirements,
	};
}

function serializeJson(data) {
	return `${JSON.stringify(data, null, 2)}\n`;
}

function serializeMarkdown(manifest) {
	const lines = [
		"# Live Evidence Instances",
		"",
		"## Status",
		"",
		`- instance_count: ${manifest.status.instance_count}`,
		`- requirements: ${manifest.status.requirements}`,
		`- missing_requirements: ${manifest.status.missing_requirements}`,
		`- missing_evidence_items: ${manifest.status.missing_evidence_items}`,
		`- decision: ${manifest.status.decision}`,
		"",
		"## Requirements",
		"",
		"| Requirement | Scope | Expected Evidence | Missing Evidence | Instances | Ready |",
		"|---|---|---|---|---:|---:|",
	];
	for (const requirement of manifest.requirements) {
		const expected = requirement.expected_evidence
			.map((entry) => `${entry.type} x${entry.min}${entry.environments ? ` (${entry.environments.join(",")})` : ""}`)
			.join(", ");
		lines.push(
			`| ${requirement.id} | ${requirement.scope} | ${expected} | ${requirement.missing_evidence.join(", ")} | ${requirement.instances.length} | ${requirement.ready} |`,
		);
	}
	lines.push("", "## Attached Instances", "");
	const instances = manifest.requirements.flatMap((requirement) =>
		requirement.instances.map((instance) => ({ requirement_id: requirement.id, ...instance })),
	);
	if (instances.length === 0) {
		lines.push("No live evidence instances attached.", "");
	} else {
		lines.push(
			"| Requirement | Evidence ID | Type | Env | Result | Decision | Source Artifact | Immutable Reference | Checksums |",
			"|---|---|---|---|---|---|---|---|---|",
		);
		for (const instance of instances) {
			lines.push(
				`| ${instance.requirement_id} | ${instance.evidence_id} | ${instance.evidence_type} | ${instance.environment} | ${instance.result} | ${instance.decision} | ${instance.source_artifact} | ${instance.immutable_reference} | ${instance.checksum_algorithms.join(", ")} |`,
			);
		}
		lines.push("");
	}
	lines.push(
		"",
		"## Validation Rules",
		"",
		"- accepted evidence must use `decision: go` with `result: passed` or `accepted`;",
		"- accepted evidence must include notes with owner justification;",
		"- `captured_at` must be parseable and cannot be in the future;",
		"- go evidence must use `ci://`, `artifact://`, `s3://`, `gs://` or `oci://` in `immutable_reference`; generic web URLs and local mutable paths are rejected;",
		"- go evidence external `source_artifact` references must use `ci://`, `artifact://`, `s3://`, `gs://` or `oci://`; generic web URLs are rejected;",
		"- accepted go evidence must use a unique `immutable_reference`; one artifact cannot satisfy multiple evidence instances;",
		"- accepted go evidence must use a unique `source_artifact`; one source artifact cannot satisfy multiple evidence instances;",
		"- accepted go evidence command must match one segment of the owning cutover packet `strict_command`; concrete run IDs may replace `<run>`;",
		"- accepted local source artifacts must be referenced by the owning packet requirement unless they are the command `--report` output;",
		"- accepted local source artifacts must stay under `docs/migration/` or the migration fixtures directory; arbitrary repository paths are rejected;",
		"- `evidence_id` must use the approved lowercase slug format and match the JSON instance filename;",
		"- when a go evidence command uses `--report`, `source_artifact` must be the same report path;",
		"- when a go evidence command uses `--env`, it must match the instance `environment`; both `--env value` and `--env=value` are supported;",
		"- go reconciliation evidence with a local JSON `source_artifact` must have a valid go or go-with-accepted-rejects reconciliation report;",
		"- go evidence must include a checksum named `source_artifact`; unrelated checksum names are rejected;",
		"- go evidence must include a checksum named `cutover_evidence_packet` matching `docs/migration/cutover-evidence-packet.generated.json`; stale packet evidence is rejected;",
		"- generation provenance must identify write and strict check commands;",
		"- requirement ids, scopes, strict commands and expected evidence must match the cutover evidence packet contract;",
		"- go evidence cannot reference `.template.` artifacts or template commands;",
		"- failed or blocking evidence must stay `decision: no-go`;",
		"- each instance must match the evidence types and environments allowed for its packet requirement;",
		"- checksum names must be unique and hexadecimal; local `source_artifact` files must match the checksum named `source_artifact`; unrelated checksums cannot satisfy source artifact proof;",
		"- placeholder IDs, owners, artifact names, references and zero checksums are rejected.",
		"",
		"## Instance Location",
		"",
		`Copy validated live evidence JSON files into \`${evidenceDir}/\`. Start from \`${templatePath}\` and keep immutable artifact references plus checksums.`,
		"",
		"## Preparation Commands",
		"",
		"Use these commands after the real run artifact exists. Replace `<run>`, `<owner>`, `<source-artifact-sha256>` and the notes before attaching the generated JSON.",
		"",
		"```bash",
		...manifest.requirements.flatMap((requirement) => requirement.preparation_commands),
		"```",
		"",
		"## Verification",
		"",
		"```bash",
		"pnpm check:migration-live-evidence-instances",
		"pnpm check:migration-live-evidence-instances -- --strict",
		"```",
		"",
	);
	return lines.join("\n");
}

const manifest = buildManifest();
const manifestJson = serializeJson(manifest);
const markdown = serializeMarkdown(manifest);
const templateJson = serializeJson(buildLiveEvidenceTemplate());

if (write) {
	mkdirSync(evidenceDir, { recursive: true });
	mkdirSync(dirname(jsonPath), { recursive: true });
	writeFileSync(templatePath, templateJson);
	writeFileSync(jsonPath, manifestJson);
	writeFileSync(markdownPath, markdown);
	console.log(`Live evidence instances written to ${markdownPath}, ${jsonPath} and ${templatePath}`);
	process.exit(0);
}

errors.push(...validateManifest(manifest, { jsonPath, strict }));
if (!existsSync(evidenceDir)) errors.push(`${evidenceDir}: missing; run tools/migration/live-evidence-instances.mjs --write`);
for (const file of nonJsonInstanceFiles()) errors.push(`${evidenceDir}/${file}: live evidence instances must be JSON files`);

for (const [path, expected] of [[templatePath, templateJson], [jsonPath, manifestJson], [markdownPath, markdown]]) {
	if (!existsSync(path)) {
		errors.push(`${path}: missing; run tools/migration/live-evidence-instances.mjs --write`);
	} else if (readFileSync(path, "utf8") !== expected) {
		errors.push(`${path}: stale; run tools/migration/live-evidence-instances.mjs --write`);
	}
}

if (errors.length > 0) {
	console.error("Live evidence instance checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(
	`Live evidence instances: ok (${manifest.status.instance_count} instances, ${manifest.status.missing_requirements} missing requirements)`,
);
