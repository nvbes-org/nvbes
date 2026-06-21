#!/usr/bin/env node
import { existsSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { spawnSync } from "node:child_process";
import { buildLiveEvidenceInstance, validatePreparedLiveEvidence } from "./live-evidence-prepare.mjs";

const instance = buildLiveEvidenceInstance([
	"--id",
	"final-reconciliation-probe",
	"--type",
	"reconciliation",
	"--env=production",
	"--owner",
	"Migration lead",
	"--source-artifact",
	"tools/migration/fixtures/reconciliation.probe.json",
	"--command",
	"node tools/migration/reconcile.mjs --env=production --report=tools/migration/fixtures/reconciliation.probe.json",
	"--result",
	"passed",
	"--decision",
	"go",
	"--immutable-reference",
	"artifact://migration/final-reconciliation-probe",
	"--packet-requirement",
	"final-reconciliation",
	"--notes",
	"Probe validates live evidence preparation with checksums.",
]);

const errors = validatePreparedLiveEvidence(instance, "tools/migration/live-evidence-prepare.check.mjs");
const checksumNames = instance.checksums.map((checksum) => checksum.name).sort();
if (checksumNames.join(",") !== "cutover_evidence_packet,source_artifact") {
	errors.push(`prepared checksums are invalid: ${checksumNames.join(",")}`);
}

const secondErrors = [];
buildLiveEvidenceInstance(["--id", "invalid-second-call"], secondErrors);
if (!secondErrors.includes("--source-artifact is required")) {
	errors.push("second build call must keep errors isolated");
}
if (!secondErrors.includes("--result is required") || !secondErrors.includes("--decision is required")) {
	errors.push("live evidence preparation must require explicit result and decision");
}

const externalErrors = [];
const externalInstance = buildLiveEvidenceInstance([
	"--id",
	"frontend-signoff-probe",
	"--type",
	"frontend_signoff",
	"--env",
	"local",
	"--owner",
	"Product lead",
	"--source-artifact",
	"artifact://migration/frontend-signoff-probe",
	"--source-checksum",
	"1".repeat(64),
	"--command",
	"pnpm check:migration-frontend-experience",
	"--result",
	"passed",
	"--decision",
	"go",
	"--immutable-reference",
	"artifact://migration/frontend-signoff-probe-result",
	"--packet-requirement",
	"g4-frontend-signoff",
], externalErrors);
errors.push(...externalErrors);
errors.push(...validatePreparedLiveEvidence(externalInstance, "external-live-evidence-probe"));

const mutableSourceErrors = [];
buildLiveEvidenceInstance([
	"--id",
	"frontend-signoff-mutable-source-probe",
	"--type",
	"frontend_signoff",
	"--env",
	"local",
	"--owner",
	"Product lead",
	"--source-artifact",
	"https://ci.example.test/frontend-signoff-probe",
	"--source-checksum",
	"1".repeat(64),
	"--command",
	"pnpm check:migration-frontend-experience",
	"--result",
	"passed",
	"--decision",
	"go",
	"--immutable-reference",
	"artifact://migration/frontend-signoff-mutable-source-probe-result",
	"--packet-requirement",
	"g4-frontend-signoff",
], mutableSourceErrors);
if (!mutableSourceErrors.some((error) => error.includes("external references must use"))) {
	errors.push("external source artifacts must use approved immutable URI schemes");
}

const zeroChecksumErrors = [];
const zeroChecksumInstance = buildLiveEvidenceInstance([
	"--id",
	"frontend-signoff-zero-checksum-probe",
	"--type",
	"frontend_signoff",
	"--env",
	"local",
	"--owner",
	"Product lead",
	"--source-artifact",
	"artifact://migration/frontend-signoff-zero-checksum-probe",
	"--source-checksum",
	"0".repeat(64),
	"--command",
	"pnpm check:migration-frontend-experience",
	"--result",
	"passed",
	"--decision",
	"go",
	"--immutable-reference",
	"artifact://migration/frontend-signoff-zero-checksum-probe-result",
	"--packet-requirement",
	"g4-frontend-signoff",
], zeroChecksumErrors);
const zeroChecksumValidation = validatePreparedLiveEvidence(zeroChecksumInstance, "zero-checksum-live-evidence-probe");
if (!zeroChecksumValidation.some((error) => error.includes("placeholder value remains"))) {
	errors.push("external source checksums cannot be zero placeholders");
}

const futureTimestampInstance = buildLiveEvidenceInstance([
	"--id",
	"frontend-signoff-future-probe",
	"--type",
	"frontend_signoff",
	"--env",
	"local",
	"--owner",
	"Product lead",
	"--source-artifact",
	"artifact://migration/frontend-signoff-future-probe",
	"--source-checksum",
	"2".repeat(64),
	"--command",
	"pnpm check:migration-frontend-experience",
	"--result",
	"passed",
	"--decision",
	"go",
	"--immutable-reference",
	"artifact://migration/frontend-signoff-future-probe-result",
	"--packet-requirement",
	"g4-frontend-signoff",
	"--captured-at",
	"2999-01-01T00:00:00.000Z",
], []);
const futureTimestampValidation = validatePreparedLiveEvidence(futureTimestampInstance, "future-live-evidence-probe");
if (!futureTimestampValidation.some((error) => error.includes("must not be in the future"))) {
	errors.push("future captured_at values must be rejected");
}

const tmp = mkdtempSync(join(tmpdir(), "nvbes-live-evidence-"));
const outputPath = join(tmp, "nested", "final-reconciliation-probe.json");
const result = spawnSync(process.execPath, [
	"tools/migration/live-evidence-prepare.mjs",
	"--id",
	"final-reconciliation-probe",
	"--type",
	"reconciliation",
	"--env=production",
	"--owner",
	"Migration lead",
	"--source-artifact",
	"tools/migration/fixtures/reconciliation.probe.json",
	"--command",
	"node tools/migration/reconcile.mjs --env=production --report=tools/migration/fixtures/reconciliation.probe.json",
	"--result",
	"passed",
	"--decision",
	"go",
	"--immutable-reference",
	"artifact://migration/final-reconciliation-probe-cli",
	"--packet-requirement",
	"final-reconciliation",
	"--notes",
	"Probe validates live evidence file output.",
	"--out",
	outputPath,
], { encoding: "utf8" });
if (result.status !== 0) errors.push(`CLI output probe failed: ${result.stderr || result.stdout}`);
if (!existsSync(outputPath)) errors.push("CLI output probe did not create nested output file");
else if (JSON.parse(readFileSync(outputPath, "utf8")).evidence_id !== "final-reconciliation-probe") {
	errors.push("CLI output probe wrote an unexpected evidence id");
}
rmSync(tmp, { recursive: true, force: true });

if (errors.length > 0) {
	console.error("Live evidence preparation checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log("Live evidence preparation: ok");
