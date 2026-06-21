#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { validateDecisionMapRows } from "./decision-map.validation.mjs";

const inventoryPath = "docs/migration/inventory.generated.json";
const outputPath = "docs/migration/secret-map.generated.json";
const markdownPath = "docs/migration/secret-map.md";
const args = process.argv.slice(2);
const write = args.includes("--write");
const strict = args.includes("--strict");
const errors = [];

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

function keyFor(entry) {
	return `${entry.source.file}#${entry.source.key}`;
}

function inferDomain(key) {
	if (/STRIPE|LAGO|BILLING|PRICE|PLAN|ENTITLEMENT/.test(key)) return "Billing/Usage";
	if (/DRIVE|UPLOAD|STORAGE|S3|BUCKET/.test(key)) return "Drive";
	if (/JWT|AUTH|OIDC|OAUTH|SAML|SESSION|MFA|WEBAUTHN/.test(key)) return "Identity";
	if (/SMTP|EMAIL|MAIL|DKIM/.test(key)) return "Email";
	if (/SENTRY|POSTHOG|ANALYTICS|OTEL|TRACE|LOG/.test(key)) return "Observability";
	if (/DATABASE|POSTGRES|REDIS|KMS|SCW|CORS|TLS|URL|PORT/.test(key)) return "Platform";
	return "Platform";
}

function inferClassification(key) {
	if (isOperationalSetting(key) || /KEY_ID|KEY_PATH|CERT_PATH|CLIENT_ID/.test(key)) return "configuration";
	if (/SECRET|TOKEN|PASSWORD|PRIVATE|DSN|WEBHOOK|JWT|KMS|AUTHORIZATION_HEADER/.test(key)) return "secret";
	if (/DATABASE|POSTGRES|REDIS|SMTP|S3|SCW|STRIPE/.test(key)) return "credential";
	if (/URL|ORIGIN|PORT|HOST|REGION|ENV|MODE/.test(key)) return "configuration";
	return "configuration";
}

function isOperationalSetting(key) {
	return /TTL|TIMEOUT|RETENTION|MAX_CONNECTIONS|PORT|ENABLED|FAIL_OPEN|SAMPLE_RATE/.test(key);
}

function isSecretLike(key) {
	if (isOperationalSetting(key) || /KEY_ID|KEY_PATH|CERT_PATH|CLIENT_ID/.test(key)) return false;
	return /SECRET|TOKEN|KEY|PASSWORD|PRIVATE|DSN|WEBHOOK|JWT|KMS|AUTHORIZATION_HEADER/.test(key);
}

function defaultVerification(key) {
	const checks = ["owner_signoff"];
	if (isSecretLike(key)) checks.push("rotation_proof");
	if (/DATABASE|POSTGRES|REDIS/.test(key)) checks.push("connectivity_smoke");
	if (/STRIPE|SMTP|S3|SCW/.test(key)) checks.push("adapter_smoke");
	return checks;
}

function slug(value) {
	return value.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "");
}

function decisionFor(key) {
	return isSecretLike(key) || /DATABASE_URL|REDIS_URL|STRIPE_SECRET|WEBHOOK_SECRET/.test(key) ? "rotate" : "keep";
}

function targetFor(key) {
	const name = slug(key);
	if (/^VITE_/.test(key)) return `deploy/oss/helm/nvbes#public-env.${name}`;
	if (decisionFor(key) === "rotate") return `deploy/oss/helm/nvbes#external-secret.${name}`;
	return `deploy/oss/helm/nvbes#config.${name}`;
}

function ownerFor(key) {
	if (decisionFor(key) === "rotate") return "Security lead";
	const domain = inferDomain(key);
	if (domain === "Billing/Usage") return "Billing lead";
	if (domain === "Drive") return "Drive lead";
	if (domain === "Identity") return "Identity lead";
	if (domain === "Email") return "Platform lead";
	if (domain === "Observability") return "Observability lead";
	return "Platform lead";
}

function rotationFor(key) {
	return decisionFor(key) === "rotate" ? "required-before-cutover" : "not-required";
}

function cutoverHandlingFor(key) {
	if (decisionFor(key) === "rotate") {
		return "provision fresh target value before cutover; revoke legacy value during decommission evidence phase";
	}
	return "carry value through target environment config with owner signoff before cutover";
}

function sourceEntries(inventory) {
	const entries = [];
	for (const fileEntry of inventory.secrets ?? []) {
		for (const key of fileEntry.keys ?? []) {
			entries.push({
				source: { file: fileEntry.file, key },
				domain: inferDomain(key),
				decision: decisionFor(key),
				target: targetFor(key),
				owner: ownerFor(key),
				classification: inferClassification(key),
				rotation: rotationFor(key),
				cutover_handling: cutoverHandlingFor(key),
				verification: defaultVerification(key),
			});
		}
	}
	return entries.sort((a, b) => keyFor(a).localeCompare(keyFor(b)));
}

function mergeExisting(generated, existing) {
	if (!existing?.entries) return generated;
	const byKey = new Map(existing.entries.map((entry) => [keyFor(entry), entry]));
	return generated.map((entry) => {
		const current = byKey.get(keyFor(entry));
		if (!current) return entry;
		return {
			...entry,
			decision: current.decision && current.decision !== "pending" ? current.decision : entry.decision,
			target: current.target && !current.target.startsWith("pending:") ? current.target : entry.target,
			owner: current.owner && current.owner !== "security lead required" ? current.owner : entry.owner,
			rotation: current.rotation && current.rotation !== "pending" ? current.rotation : entry.rotation,
			cutover_handling: current.cutover_handling && !current.cutover_handling.startsWith("blocking until")
				? current.cutover_handling
				: entry.cutover_handling,
			verification: current.verification ?? entry.verification,
		};
	});
}

function serialize(data) {
	const lines = [
		"{",
		`  "schema_version": ${JSON.stringify(data.schema_version)},`,
		`  "generation": ${JSON.stringify(data.generation)},`,
		`  "summary": ${JSON.stringify(data.summary)},`,
		'  "entries": [',
	];
	for (const [index, entry] of data.entries.entries()) {
		const suffix = index === data.entries.length - 1 ? "" : ",";
		lines.push(`    ${JSON.stringify(entry)}${suffix}`);
	}
	lines.push("  ]", "}");
	return `${lines.join("\n")}\n`;
}

function serializeMarkdown(data) {
	const lines = [
		"# Secret Migration Map",
		"",
		"## Status",
		"",
		`- entries: ${data.summary.entries}`,
		`- pending: ${data.summary.pending}`,
		`- keep: ${data.summary.keep}`,
		`- rotate: ${data.summary.rotate}`,
		`- remove: ${data.summary.remove}`,
		`- replace: ${data.summary.replace}`,
		"",
		"## Rules",
		"",
		"- Every secret or config key must have a target, owner, rotation decision and verification checks.",
		"- Source rows, domains and summary counters must match the inventory-derived map.",
		"- Generation provenance must identify source, write command and strict cutover command.",
		"",
		"## Secrets",
		"",
		"| Key | Domain | Decision | Owner | Rotation | Target |",
		"|---|---|---:|---|---|---|",
	];
	for (const entry of data.entries) {
		lines.push(
			`| ${entry.source.key} | ${entry.domain} | ${entry.decision} | ${entry.owner} | ${entry.rotation} | \`${entry.target}\` |`,
		);
	}
	lines.push(
		"",
		"## Regeneration",
		"",
		"```bash",
		"pnpm check:migration-secret-map",
		"tools/migration/secret-map.mjs --write",
		"```",
		"",
	);
	return lines.join("\n");
}

function validate(map, expectedEntries) {
	const expectedKeys = new Set(expectedEntries.map(keyFor));
	const seen = new Set();
	if (map.schema_version !== 1) errors.push(`${outputPath}: schema_version must be 1`);
	if (map.generation?.command !== "tools/migration/secret-map.mjs --write") errors.push(`${outputPath}: generation.command is invalid`);
	if (map.generation?.source !== inventoryPath) errors.push(`${outputPath}: generation.source is invalid`);
	if (map.generation?.strict_cutover_command !== "tools/migration/secret-map.mjs --strict") errors.push(`${outputPath}: generation.strict_cutover_command is invalid`);
	if (!Array.isArray(map.entries)) errors.push(`${outputPath}: entries must be an array`);
	validateDecisionMapRows({ map, expectedEntries, keyFor, outputPath, errors });
	for (const entry of map.entries ?? []) {
		const key = keyFor(entry);
		if (seen.has(key)) errors.push(`${outputPath}: duplicate entry ${key}`);
		seen.add(key);
		if (!expectedKeys.has(key)) errors.push(`${outputPath}: stale entry ${key}`);
		if (!entry.domain) errors.push(`${key}: domain is required`);
		if (!["pending", "keep", "rotate", "remove", "replace"].includes(entry.decision)) errors.push(`${key}: unsupported decision ${entry.decision}`);
		if (!entry.target) errors.push(`${key}: target is required`);
		if (!entry.owner) errors.push(`${key}: owner is required`);
		if (!entry.classification) errors.push(`${key}: classification is required`);
		if (!entry.rotation) errors.push(`${key}: rotation is required`);
		if (!entry.cutover_handling) errors.push(`${key}: cutover_handling is required`);
		if (!Array.isArray(entry.verification) || entry.verification.length === 0) errors.push(`${key}: verification checks are required`);
		if (strict) {
			if (entry.decision === "pending") errors.push(`${key}: pending decision blocks cutover`);
			if (entry.owner === "security lead required") errors.push(`${key}: owner must be assigned`);
			if (entry.target.startsWith("pending:")) errors.push(`${key}: target must be resolved`);
			if (entry.rotation === "pending") errors.push(`${key}: rotation must be resolved`);
		}
	}
	for (const key of expectedKeys) {
		if (!seen.has(key)) errors.push(`${outputPath}: missing entry ${key}`);
	}
}

const inventory = readJson(inventoryPath);
const generatedEntries = inventory ? sourceEntries(inventory) : [];
const existing = existsSync(outputPath) ? readJson(outputPath) : undefined;
const secretMap = {
	schema_version: 1,
	generation: {
		command: "tools/migration/secret-map.mjs --write",
		source: inventoryPath,
		strict_cutover_command: "tools/migration/secret-map.mjs --strict",
	},
	summary: { entries: generatedEntries.length, pending: 0, keep: 0, rotate: 0, remove: 0, replace: 0 },
	entries: mergeExisting(generatedEntries, existing),
};

for (const entry of secretMap.entries) {
	secretMap.summary[entry.decision] += 1;
}

if (write) {
	mkdirSync(dirname(outputPath), { recursive: true });
	writeFileSync(outputPath, serialize(secretMap));
	writeFileSync(markdownPath, serializeMarkdown(secretMap));
	console.log(`Migration secret map written to ${markdownPath} and ${outputPath}`);
	process.exit(0);
}

validate(secretMap, generatedEntries);

if (!existsSync(outputPath)) {
	errors.push(`${outputPath}: missing; run tools/migration/secret-map.mjs --write`);
} else if (readFileSync(outputPath, "utf8") !== serialize(secretMap)) {
	errors.push(`${outputPath}: stale; run tools/migration/secret-map.mjs --write`);
}

if (!existsSync(markdownPath)) {
	errors.push(`${markdownPath}: missing; run tools/migration/secret-map.mjs --write`);
} else if (readFileSync(markdownPath, "utf8") !== serializeMarkdown(secretMap)) {
	errors.push(`${markdownPath}: stale; run tools/migration/secret-map.mjs --write`);
}

if (errors.length > 0) {
	console.error("Migration secret-map checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(`Migration secret map: ok (${secretMap.summary.entries} entries, ${secretMap.summary.pending} pending)`);
