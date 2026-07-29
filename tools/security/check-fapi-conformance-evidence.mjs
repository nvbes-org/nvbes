import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";

const enabled = process.env.NVBES_FAPI_HIGH_ASSURANCE_ENABLED === "true";
if (!enabled) {
  console.log("FAPI high-assurance profile is disabled; no conformance evidence is required.");
  process.exit(0);
}

const evidencePath = process.env.NVBES_FAPI_CONFORMANCE_EVIDENCE_FILE;
const expectedDigest = process.env.NVBES_FAPI_CONFORMANCE_EVIDENCE_SHA256;
if (!evidencePath || !expectedDigest) {
  throw new Error(
    "Enabled FAPI high assurance requires NVBES_FAPI_CONFORMANCE_EVIDENCE_FILE and NVBES_FAPI_CONFORMANCE_EVIDENCE_SHA256.",
  );
}

const raw = await readFile(evidencePath);
const digest = createHash("sha256").update(raw).digest("hex");
if (digest.toLowerCase() !== expectedDigest.toLowerCase()) {
  throw new Error("FAPI conformance evidence SHA-256 does not match the approved digest.");
}

const evidence = JSON.parse(raw.toString("utf8"));
if (evidence.profile !== "fapi2-security-profile-final") {
  throw new Error("Evidence must target the final FAPI 2.0 Security Profile.");
}
if (evidence.status !== "passed") {
  throw new Error("Every imported FAPI conformance plan must have passed.");
}
if (!evidence.deployment_commit || !evidence.suite_version || !evidence.completed_at) {
  throw new Error("Evidence must identify the deployment commit, suite version, and completion time.");
}
if (!Array.isArray(evidence.plans) || evidence.plans.length === 0) {
  throw new Error("Evidence must contain at least one official conformance plan.");
}
for (const plan of evidence.plans) {
  if (
    plan.plan_name !== "fapi2-security-profile-final-test-plan" ||
    plan.status !== "passed" ||
    !plan.result_url
  ) {
    throw new Error("Each plan must be a passed official FAPI 2.0 final plan with a result URL.");
  }
}

console.log(`Verified FAPI conformance evidence ${digest}.`);
