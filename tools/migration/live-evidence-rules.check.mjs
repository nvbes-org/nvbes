#!/usr/bin/env node
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import {
	buildLiveEvidenceTemplate,
	validateLiveEvidenceInstance,
	validateLiveEvidenceInstanceSet,
} from "./live-evidence.rules.mjs";
import { validateManifest } from "./live-evidence-instances.validation.mjs";

const schemaPath = "docs/migration/live-evidence.schema.json";
const sourceArtifact = "tools/migration/fixtures/reconciliation.probe.json";
const blockingArtifact = "tools/migration/fixtures/reconciliation.blocking.probe.json";
const alternateArtifact = "docs/migration/reconciliation-report.schema.json";
const templateArtifact = "docs/migration/reconciliation.template.json";
const packetArtifact = "docs/migration/cutover-evidence-packet.generated.json";
const frontendArtifact = "docs/migration/frontend-experience.generated.json";
const schema = JSON.parse(readFileSync(schemaPath, "utf8"));
const packet = JSON.parse(readFileSync(packetArtifact, "utf8"));
const manifest = JSON.parse(readFileSync("docs/migration/live-evidence-instances.generated.json", "utf8"));
const sourceDigest = createHash("sha256").update(readFileSync(sourceArtifact)).digest("hex");
const blockingDigest = createHash("sha256").update(readFileSync(blockingArtifact)).digest("hex");
const alternateDigest = createHash("sha256").update(readFileSync(alternateArtifact)).digest("hex");
const templateDigest = createHash("sha256").update(readFileSync(templateArtifact)).digest("hex");
const packetDigest = createHash("sha256").update(readFileSync(packetArtifact)).digest("hex");
const frontendDigest = createHash("sha256").update(readFileSync(frontendArtifact)).digest("hex");
const fixturePacket = {
	...packet,
	requirements: packet.requirements.map((requirement) =>
		requirement.id === "final-reconciliation"
			? {
					...requirement,
					strict_command: "node tools/migration/reconcile.mjs --env production --report tools/migration/fixtures/reconciliation.<run>.json",
				}
			: requirement,
	),
};
const errors = [];
let probes = 0;

function validInstance(overrides = {}) {
	return {
		schema_version: 1,
		evidence_id: "probe-live-evidence",
		evidence_type: "reconciliation",
		environment: "production",
		captured_at: new Date(Date.now() - 60_000).toISOString(),
		owner: "Data lead",
		source_artifact: sourceArtifact,
		command: "node tools/migration/reconcile.mjs --env production --report tools/migration/fixtures/reconciliation.probe.json",
		result: "passed",
		decision: "go",
		immutable_reference: "ci://probe/live-evidence",
		packet_requirement: "final-reconciliation",
		checksums: [
			{ name: "source_artifact", algorithm: "sha256", value: sourceDigest },
			{ name: "cutover_evidence_packet", algorithm: "sha256", value: packetDigest },
		],
		notes: "Synthetic validator probe.",
		...overrides,
	};
}

function frontendInstance(source_artifact, digest) {
	return validInstance({
		evidence_id: "frontend-live-evidence",
		evidence_type: "frontend_signoff",
		source_artifact,
		command: "pnpm check:migration-frontend-experience",
		packet_requirement: "g4-frontend-signoff",
		checksums: [
			{ name: "source_artifact", algorithm: "sha256", value: digest },
			{ name: "cutover_evidence_packet", algorithm: "sha256", value: packetDigest },
		],
	});
}

function record(path, instance) { return { path, instance }; }
function expectValid(label, instance, options = {}) {
	probes += 1;
	const validationErrors = validateLiveEvidenceInstance(instance, schema, `${label}.json`, options);
	if (validationErrors.length > 0) {
		errors.push(`${label}: expected valid, got ${validationErrors.join("; ")}`);
	}
}

function expectInvalid(label, instance, expectedFragment) {
	probes += 1;
	const validationErrors = validateLiveEvidenceInstance(instance, schema, `${label}.json`);
	if (!validationErrors.some((error) => error.includes(expectedFragment))) {
		errors.push(`${label}: expected error containing "${expectedFragment}", got ${validationErrors.join("; ") || "none"}`);
	}
}

function expectSetValid(label, records, packetOverride = packet) {
	probes += 1;
	const validationErrors = validateLiveEvidenceInstanceSet(records, packetOverride);
	if (validationErrors.length > 0) {
		errors.push(`${label}: expected valid, got ${validationErrors.join("; ")}`);
	}
}

function expectSetInvalid(label, records, expectedFragment, packetOverride = packet) {
	probes += 1;
	const validationErrors = validateLiveEvidenceInstanceSet(records, packetOverride);
	if (!validationErrors.some((error) => error.includes(expectedFragment))) {
		errors.push(`${label}: expected error containing "${expectedFragment}", got ${validationErrors.join("; ") || "none"}`);
	}
}
function expectManifestInvalid(label, mutate, expectedFragment) {
	probes += 1;
	const copy = JSON.parse(JSON.stringify(manifest)); mutate(copy);
	const validationErrors = validateManifest(copy, { jsonPath: "docs/migration/live-evidence-instances.generated.json", strict: false });
	if (!validationErrors.some((error) => error.includes(expectedFragment))) errors.push(`${label}: expected error containing "${expectedFragment}", got ${validationErrors.join("; ") || "none"}`);
}

expectValid("valid-local-artifact", validInstance());
expectValid("template", buildLiveEvidenceTemplate(), { allowTemplate: true });
expectInvalid("bad-local-digest", validInstance({ checksums: [{ name: "source_artifact", algorithm: "sha256", value: "f".repeat(64) }, { name: "cutover_evidence_packet", algorithm: "sha256", value: packetDigest }] }), "checksum");
expectInvalid("go-with-blocking-result", validInstance({ result: "blocking" }), "decision go requires result");
expectInvalid("failed-with-go-decision", validInstance({ result: "failed" }), "failed or blocking result");
expectInvalid("go-with-local-immutable-reference", validInstance({ immutable_reference: "docs/migration/local-artifact.json" }), "approved immutable URI scheme");
expectInvalid("go-with-generic-https-source-artifact", validInstance({ source_artifact: "https://ci.example.test/artifacts/live-evidence" }), "source_artifact");
expectInvalid(
	"go-without-source-artifact-checksum",
	validInstance({
		checksums: [
			{ name: "unrelated", algorithm: "sha256", value: sourceDigest },
			{ name: "cutover_evidence_packet", algorithm: "sha256", value: packetDigest },
		],
	}),
	"checksum named source_artifact",
);
expectInvalid(
	"stale-source-artifact-not-masked-by-unrelated-checksum",
	validInstance({
		checksums: [
			{ name: "source_artifact", algorithm: "sha256", value: "f".repeat(64) },
			{ name: "unrelated", algorithm: "sha256", value: sourceDigest },
			{ name: "cutover_evidence_packet", algorithm: "sha256", value: packetDigest },
		],
	}),
	"checksum named source_artifact must match",
);
expectInvalid(
	"duplicate-checksum-name",
	validInstance({
		checksums: [
			{ name: "source_artifact", algorithm: "sha256", value: sourceDigest },
			{ name: "source_artifact", algorithm: "sha256", value: sourceDigest },
			{ name: "cutover_evidence_packet", algorithm: "sha256", value: packetDigest },
		],
	}),
	"duplicate checksum name source_artifact",
);
expectInvalid("too-many-checksums", validInstance({ checksums: Array.from({ length: 5 }, (_, index) => ({ name: `extra_${index}`, algorithm: "sha256", value: sourceDigest })) }), "at most 4 entries");
expectInvalid("go-without-packet-checksum", validInstance({ checksums: [{ name: "source_artifact", algorithm: "sha256", value: sourceDigest }] }), "checksum named cutover_evidence_packet");
expectInvalid(
	"go-with-stale-packet-checksum",
	validInstance({
		checksums: [
			{ name: "source_artifact", algorithm: "sha256", value: sourceDigest },
			{ name: "cutover_evidence_packet", algorithm: "sha256", value: "f".repeat(64) },
		],
	}),
	"cutover_evidence_packet checksum",
);
expectInvalid("local-source-artifact-outside-approved-roots", validInstance({ source_artifact: "docs/../package.json" }), "approved migration evidence paths");
expectInvalid(
	"non-hex-checksum",
	validInstance({
		checksums: [
			{ name: "source_artifact", algorithm: "sha256", value: `${sourceDigest.slice(0, 63)}z` },
			{ name: "cutover_evidence_packet", algorithm: "sha256", value: packetDigest },
		],
	}),
	"checksum value must be hexadecimal",
);
expectInvalid("future-captured-at", validInstance({ captured_at: new Date(Date.now() + 3_600_000).toISOString() }), "captured_at must not be in the future");
expectInvalid(
	"go-with-template-source-artifact",
	validInstance({
		source_artifact: templateArtifact,
		command: "node tools/migration/reconcile.mjs --env production --report docs/migration/reconciliation.template.json",
		checksums: [
			{ name: "source_artifact", algorithm: "sha256", value: templateDigest },
			{ name: "cutover_evidence_packet", algorithm: "sha256", value: packetDigest },
		],
	}),
	"template artifacts",
);
expectInvalid(
	"source-artifact-report-mismatch",
	validInstance({
		source_artifact: alternateArtifact,
		checksums: [
			{ name: "source_artifact", algorithm: "sha256", value: alternateDigest },
			{ name: "cutover_evidence_packet", algorithm: "sha256", value: packetDigest },
		],
	}),
	"source_artifact must match command --report path",
);
expectInvalid(
	"blocking-reconciliation-report",
	validInstance({
		source_artifact: blockingArtifact,
		command: "node tools/migration/reconcile.mjs --env production --report tools/migration/fixtures/reconciliation.blocking.probe.json",
		checksums: [
			{ name: "source_artifact", algorithm: "sha256", value: blockingDigest },
			{ name: "cutover_evidence_packet", algorithm: "sha256", value: packetDigest },
		],
	}),
	"reconciliation report decision must be go or go-with-accepted-rejects",
);
expectInvalid(
	"command-env-mismatch",
	validInstance({
		command: "node tools/migration/reconcile.mjs --env staging --report tools/migration/fixtures/reconciliation.probe.json",
	}),
	"environment must match command --env value",
);
expectValid(
	"inline-env-and-report-options",
	validInstance({
		command: "node tools/migration/reconcile.mjs --env=production --report=tools/migration/fixtures/reconciliation.probe.json",
	}),
);
const acceptedWithoutNotes = validInstance({ result: "accepted" });
delete acceptedWithoutNotes.notes;
expectInvalid("accepted-without-notes", acceptedWithoutNotes, "accepted result requires notes");
expectValid("accepted-with-notes", validInstance({ result: "accepted", notes: "Owner accepted documented reconciliation delta." }));
const missingPacketRequirement = validInstance();
delete missingPacketRequirement.packet_requirement;
expectInvalid("missing-packet-requirement", missingPacketRequirement, "missing required field packet_requirement");
const missingSchemaVersion = validInstance();
delete missingSchemaVersion.schema_version;
expectInvalid("missing-schema-version", missingSchemaVersion, "missing required field schema_version");
expectInvalid("wrong-schema-version", validInstance({ schema_version: 2 }), "schema_version must be 1");
expectInvalid("bad-evidence-id-format", validInstance({ evidence_id: "Probe live evidence" }), "evidence_id must match");
expectInvalid("unknown-packet-requirement", validInstance({ packet_requirement: "unknown-requirement" }), "unknown packet_requirement");
expectInvalid("wrong-evidence-type", validInstance({ evidence_type: "cutover" }), "is not valid for final-reconciliation");
expectInvalid("wrong-requirement-environment", validInstance({ environment: "staging" }), "environment staging is not valid");
expectSetInvalid(
	"duplicate-accepted-immutable-reference",
	[record("first-live-evidence.json", validInstance({ evidence_id: "first-live-evidence" })), record("second-live-evidence.json", validInstance({ evidence_id: "second-live-evidence" }))],
	"duplicate accepted immutable_reference",
);
expectSetInvalid(
	"duplicate-accepted-source-artifact",
	[record("first-live-evidence.json", validInstance({ evidence_id: "first-live-evidence", immutable_reference: "ci://probe/first" })), record("second-live-evidence.json", validInstance({ evidence_id: "second-live-evidence", immutable_reference: "ci://probe/second" }))],
	"duplicate accepted source_artifact",
);
expectSetValid("strict-command-with-run-substitution", [record("probe-live-evidence.json", validInstance())], fixturePacket);
expectSetInvalid(
	"command-not-in-strict-command",
	[record("probe-live-evidence.json", validInstance({ command: "pnpm check" }))],
	"command must match strict_command",
	fixturePacket,
);
expectSetInvalid(
	"strict-command-run-placeholder-rejects-path",
	[record("probe-live-evidence.json", validInstance({ command: "node tools/migration/reconcile.mjs --env production --report tools/migration/fixtures/reconciliation../probe.json" }))],
	"command must match strict_command",
	fixturePacket,
);
expectSetInvalid(
	"local-source-artifact-not-in-packet-files",
	[record("frontend-live-evidence.json", frontendInstance(alternateArtifact, alternateDigest))],
	"local source_artifact must be referenced by the packet requirement",
);
expectSetValid(
	"local-source-artifact-in-packet-files",
	[record("frontend-live-evidence.json", frontendInstance(frontendArtifact, frontendDigest))],
	packet,
);
expectSetInvalid(
	"evidence-id-filename-mismatch",
	[record("wrong-filename.json", validInstance())],
	"evidence_id must match instance filename",
);
expectInvalid("placeholder-owner", validInstance({ owner: "migration lead required" }), "placeholder value remains");
expectInvalid(
	"zero-checksum",
	validInstance({
		checksums: [
			{ name: "source_artifact", algorithm: "sha256", value: "0".repeat(64) },
			{ name: "cutover_evidence_packet", algorithm: "sha256", value: packetDigest },
		],
	}),
	"placeholder value remains",
);
expectManifestInvalid("manifest-derived-missing-types", (copy) => { copy.requirements[0].missing_evidence_types = ["wrong"]; }, "missing_evidence_types must match");
expectManifestInvalid("manifest-derived-accepted-types", (copy) => { copy.requirements[0].accepted_evidence_types = ["wrong"]; }, "accepted_evidence_types must match");
if (errors.length > 0) {
	console.error("Live evidence rule checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}
console.log(`Live evidence rule checks: ok (${probes} probes)`);
