import { createHash, verify } from "node:crypto";
import { createReadStream } from "node:fs";
import { lstat, readFile, realpath } from "node:fs/promises";
import path from "node:path";
import { pathToFileURL } from "node:url";
import { verifyDeploymentAttestation } from "./verify-deployment-attestation.mjs";

export const REQUIRED_HUMAN_CATEGORIES = Object.freeze([
	"acceptance-alpha",
	"acceptance-beta",
	"beta",
	"user-acceptance",
	"usability",
	"exploratory",
	"black-box",
	"penetration",
	"compliance",
	"recovery",
	"drp-failover",
	"localization",
]);

const MAX_MANIFEST_BYTES = 1024 * 1024;
const MAX_REPORT_BYTES = 100 * 1024 * 1024;
const MAX_VALIDITY_MS = 31 * 24 * 60 * 60 * 1000;
const REPORT_MAX_AGE_MS = Object.freeze({
	"acceptance-alpha": 7 * 24 * 60 * 60 * 1000,
	"acceptance-beta": 14 * 24 * 60 * 60 * 1000,
	beta: 14 * 24 * 60 * 60 * 1000,
	"black-box": 7 * 24 * 60 * 60 * 1000,
	compliance: 7 * 24 * 60 * 60 * 1000,
	"drp-failover": 7 * 24 * 60 * 60 * 1000,
	exploratory: 7 * 24 * 60 * 60 * 1000,
	localization: 7 * 24 * 60 * 60 * 1000,
	penetration: 31 * 24 * 60 * 60 * 1000,
	recovery: 7 * 24 * 60 * 60 * 1000,
	usability: 7 * 24 * 60 * 60 * 1000,
	"user-acceptance": 7 * 24 * 60 * 60 * 1000,
});
const RELEASE_PATTERN = /^[a-f0-9]{40,64}$/u;
const SHA256_PATTERN = /^[a-f0-9]{64}$/u;

export async function verifyAcceptanceEvidence({
	evidencePath,
	signaturePath,
	publicKeyPath,
	expectedPublicKeySha256,
	expectedRelease,
	expectedSignerKeyId,
	deploymentTrust,
	now = new Date(),
}) {
	assert(
		RELEASE_PATTERN.test(expectedRelease ?? ""),
		"expected release must be an immutable SHA",
	);
	assert(
		SHA256_PATTERN.test(expectedPublicKeySha256 ?? ""),
		"trusted public key SHA-256 is required",
	);
	assert(nonEmpty(expectedSignerKeyId), "trusted signer key id is required");
	const evidenceFile = await readBoundedRegularFile(
		evidencePath,
		MAX_MANIFEST_BYTES,
		"evidence manifest",
	);
	const signature = await readBoundedRegularFile(
		signaturePath,
		4096,
		"detached signature",
	);
	const publicKey = await readBoundedRegularFile(
		publicKeyPath,
		16_384,
		"trusted public key",
	);
	assert(
		sha256Bytes(publicKey.content) === expectedPublicKeySha256,
		"trusted public key fingerprint mismatch",
	);
	assert(
		verify(null, evidenceFile.content, publicKey.content, signature.content),
		"acceptance evidence signature is invalid",
	);

	const manifest = parseManifest(evidenceFile.content);
	const issuedAt = validateEnvelope(
		manifest,
		expectedRelease,
		expectedSignerKeyId,
		now,
	);
	const evidenceRoot = await realpath(path.dirname(evidenceFile.path));
	await validateReports(manifest.reports, evidenceRoot, {
		environment: manifest.environment,
		expectedRelease,
		issuedAt,
		now,
	});
	await verifyDeploymentAttestation({
		deployment: manifest.deployment,
		evidenceRoot,
		expectedRelease,
		now,
		trust: deploymentTrust,
	});

	return {
		categories: REQUIRED_HUMAN_CATEGORIES.length,
		deploymentRelease: expectedRelease,
		release: manifest.release,
		signerKeyId: manifest.signerKeyId,
	};
}

function parseManifest(content) {
	let manifest;
	try {
		manifest = JSON.parse(content.toString("utf8"));
	} catch {
		throw new Error("acceptance evidence manifest is not valid JSON");
	}
	assert(isRecord(manifest), "acceptance evidence manifest must be an object");
	return manifest;
}

function validateEnvelope(manifest, expectedRelease, expectedSignerKeyId, now) {
	assert(
		manifest.schemaVersion === 1,
		"unsupported acceptance evidence schema",
	);
	assert(
		manifest.environment === "staging",
		"acceptance evidence must come from staging",
	);
	assert(
		manifest.release === expectedRelease,
		"acceptance evidence release mismatch",
	);
	assert(
		typeof manifest.signerKeyId === "string" &&
			/^[a-z0-9][a-z0-9._-]{2,127}$/u.test(manifest.signerKeyId),
		"acceptance evidence signer key id is invalid",
	);
	assert(
		manifest.signerKeyId === expectedSignerKeyId,
		"acceptance evidence signer key id is not trusted",
	);
	const issuedAt = timestamp(manifest.issuedAt, "issuedAt");
	const expiresAt = timestamp(manifest.expiresAt, "expiresAt");
	const nowMs = now.getTime();
	assert(
		issuedAt <= nowMs + 5 * 60 * 1000,
		"acceptance evidence is future-dated",
	);
	assert(expiresAt > nowMs, "acceptance evidence is expired");
	assert(expiresAt > issuedAt, "acceptance evidence expiry precedes issuance");
	assert(
		expiresAt - issuedAt <= MAX_VALIDITY_MS,
		"acceptance evidence validity exceeds 31 days",
	);
	assert(
		Array.isArray(manifest.reports),
		"acceptance evidence reports must be an array",
	);
	return issuedAt;
}

async function validateReports(
	reports,
	evidenceRoot,
	{ environment, expectedRelease, issuedAt, now },
) {
	assert(
		reports.length === REQUIRED_HUMAN_CATEGORIES.length,
		"acceptance evidence must contain every required human report exactly once",
	);
	const observed = new Set();
	for (const report of reports) {
		assert(isRecord(report), "acceptance evidence report must be an object");
		assert(
			REQUIRED_HUMAN_CATEGORIES.includes(report.category),
			`unexpected acceptance evidence category ${String(report.category)}`,
		);
		assert(
			!observed.has(report.category),
			`duplicate acceptance evidence category ${report.category}`,
		);
		observed.add(report.category);
		assert(
			report.release === expectedRelease,
			`${report.category} report release mismatch`,
		);
		assert(
			report.environment === environment,
			`${report.category} report environment mismatch`,
		);
		assert(
			report.decision === "passed",
			`${report.category} decision must be passed`,
		);
		assert(
			nonEmpty(report.signedBy),
			`${report.category} must name its human signer`,
		);
		const signedAt = timestamp(report.signedAt, `${report.category}.signedAt`);
		assert(
			signedAt <= issuedAt,
			`${report.category} was signed after issuance`,
		);
		assert(
			signedAt <= now.getTime() + 5 * 60 * 1000,
			`${report.category} is future-dated`,
		);
		assert(
			issuedAt - signedAt <= REPORT_MAX_AGE_MS[report.category],
			`${report.category} report is older than its freshness policy`,
		);
		if (report.category === "penetration") {
			assert(
				report.independent === true,
				"penetration evidence must be independent",
			);
		}
		await verifyArtifact(report, evidenceRoot, `${report.category} report`);
	}
}

async function verifyArtifact(entry, evidenceRoot, label) {
	assert(nonEmpty(entry.path), `${label} path is required`);
	assert(
		SHA256_PATTERN.test(entry.sha256 ?? ""),
		`${label} SHA-256 is invalid`,
	);
	const artifactRealPath = await resolveEvidenceFile(
		entry.path,
		evidenceRoot,
		MAX_REPORT_BYTES,
		label,
	);
	assert(
		(await sha256File(artifactRealPath)) === entry.sha256,
		`${label} digest mismatch`,
	);
	return artifactRealPath;
}

async function resolveEvidenceFile(
	relativePath,
	evidenceRoot,
	maxBytes,
	label,
) {
	assert(nonEmpty(relativePath), `${label} path is required`);
	const candidate = path.resolve(evidenceRoot, relativePath);
	assert(
		candidate.startsWith(`${evidenceRoot}${path.sep}`),
		`${label} path escapes the evidence directory`,
	);
	const artifact = await readRegularFileMetadata(candidate, maxBytes, label);
	const artifactRealPath = await realpath(artifact.path);
	assert(
		artifactRealPath.startsWith(`${evidenceRoot}${path.sep}`),
		`${label} resolves outside the evidence directory`,
	);
	return artifactRealPath;
}

async function readBoundedRegularFile(filePath, maxBytes, label) {
	const metadata = await readRegularFileMetadata(filePath, maxBytes, label);
	return { ...metadata, content: await readFile(metadata.path) };
}

async function readRegularFileMetadata(filePath, maxBytes, label) {
	assert(nonEmpty(filePath), `${label} path is required`);
	const resolved = path.resolve(filePath);
	const metadata = await lstat(resolved);
	assert(
		metadata.isFile() && !metadata.isSymbolicLink(),
		`${label} must be a regular file`,
	);
	assert(
		metadata.size > 0 && metadata.size <= maxBytes,
		`${label} has an invalid size`,
	);
	return { path: resolved };
}

async function sha256File(filePath) {
	const hash = createHash("sha256");
	for await (const chunk of createReadStream(filePath)) {
		hash.update(chunk);
	}
	return hash.digest("hex");
}

function sha256Bytes(content) {
	return createHash("sha256").update(content).digest("hex");
}

function timestamp(value, label) {
	assert(
		typeof value === "string" &&
			/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$/u.test(value),
		`${label} must be a canonical UTC ISO timestamp`,
	);
	const parsed = Date.parse(value);
	assert(
		Number.isFinite(parsed) && new Date(parsed).toISOString() === value,
		`${label} must be a canonical UTC ISO timestamp`,
	);
	return parsed;
}

function nonEmpty(value) {
	return (
		typeof value === "string" && value.trim().length > 0 && value.length <= 256
	);
}

function isRecord(value) {
	return typeof value === "object" && value !== null && !Array.isArray(value);
}

function assert(condition, message) {
	if (!condition) {
		throw new Error(message);
	}
}

async function main() {
	const result = await verifyAcceptanceEvidence({
		evidencePath: process.env.ACCOUNT_ACCEPTANCE_EVIDENCE_FILE,
		deploymentTrust: {
			expectedComponentUids: parseExpectedComponentUids(
				process.env.ACCOUNT_DEPLOYMENT_EXPECTED_COMPONENT_UIDS,
			),
			expectedPublicKeySha256:
				process.env.ACCOUNT_DEPLOYMENT_TRUSTED_PUBLIC_KEY_SHA256,
			expectedSignerKeyId: process.env.ACCOUNT_DEPLOYMENT_TRUSTED_KEY_ID,
			publicKeyPath: process.env.ACCOUNT_DEPLOYMENT_TRUSTED_PUBLIC_KEY_FILE,
		},
		expectedPublicKeySha256:
			process.env.ACCOUNT_ACCEPTANCE_TRUSTED_PUBLIC_KEY_SHA256,
		expectedRelease: process.env.NVBES_RELEASE_SHA,
		expectedSignerKeyId: process.env.ACCOUNT_ACCEPTANCE_TRUSTED_KEY_ID,
		publicKeyPath: process.env.ACCOUNT_ACCEPTANCE_TRUSTED_PUBLIC_KEY_FILE,
		signaturePath: process.env.ACCOUNT_ACCEPTANCE_EVIDENCE_SIGNATURE_FILE,
	});
	process.stdout.write(
		`Verified signed Account acceptance evidence for ${result.release} (${result.categories} human gates plus production deployment).\n`,
	);
}

function parseExpectedComponentUids(source) {
	if (!source) {
		throw new Error("ACCOUNT_DEPLOYMENT_EXPECTED_COMPONENT_UIDS is required");
	}
	try {
		return JSON.parse(source);
	} catch {
		throw new Error(
			"ACCOUNT_DEPLOYMENT_EXPECTED_COMPONENT_UIDS must be valid JSON",
		);
	}
}

if (
	process.argv[1] &&
	import.meta.url === pathToFileURL(process.argv[1]).href
) {
	main().catch((error) => {
		process.stderr.write(
			`${error instanceof Error ? error.message : "acceptance evidence verification failed"}\n`,
		);
		process.exitCode = 1;
	});
}
