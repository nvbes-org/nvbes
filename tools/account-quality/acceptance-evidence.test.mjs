import assert from "node:assert/strict";
import { createHash, generateKeyPairSync, sign } from "node:crypto";
import {
	mkdir,
	mkdtemp,
	readFile,
	rm,
	symlink,
	writeFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";

import {
	REQUIRED_HUMAN_CATEGORIES,
	verifyAcceptanceEvidence,
} from "./verify-acceptance-evidence.mjs";

const RELEASE = "a".repeat(40);
const NOW = new Date("2026-07-30T12:00:00.000Z");

test("accepts a signed, release-bound, complete evidence packet", async (t) => {
	const fixture = await evidenceFixture(t);
	const result = await fixture.verify();
	assert.deepEqual(result, {
		categories: REQUIRED_HUMAN_CATEGORIES.length,
		deploymentRelease: RELEASE,
		release: RELEASE,
		signerKeyId: "account-release-board-2026",
	});
});

test("rejects tampered report bytes", async (t) => {
	const fixture = await evidenceFixture(t);
	await writeFile(
		path.join(fixture.root, fixture.manifest.reports[0].path),
		"tampered",
	);
	await assert.rejects(fixture.verify(), /digest mismatch/u);
});

test("rejects a missing human gate even with a valid packet signature", async (t) => {
	const fixture = await evidenceFixture(t);
	fixture.manifest.reports.pop();
	await fixture.persist();
	await assert.rejects(fixture.verify(), /every required human report/u);
});

test("rejects manifest tampering after signature", async (t) => {
	const fixture = await evidenceFixture(t);
	fixture.manifest.release = "b".repeat(40);
	await writeFile(
		fixture.evidencePath,
		`${JSON.stringify(fixture.manifest, null, 2)}\n`,
	);
	await assert.rejects(fixture.verify(), /signature is invalid/u);
});

test("rejects a swapped trust root or signer identity", async (t) => {
	const fixture = await evidenceFixture(t);
	await assert.rejects(
		fixture.verify({ expectedPublicKeySha256: "b".repeat(64) }),
		/fingerprint mismatch/u,
	);
	await assert.rejects(
		fixture.verify({ expectedSignerKeyId: "untrusted-release-board" }),
		/key id is not trusted/u,
	);
	await assert.rejects(
		fixture.verify({
			deploymentTrust: {
				...fixture.deploymentTrust,
				expectedPublicKeySha256: "b".repeat(64),
			},
		}),
		/deployment trusted public key fingerprint mismatch/u,
	);
});

test("rejects expired and cross-release evidence", async (t) => {
	const fixture = await evidenceFixture(t);
	fixture.manifest.expiresAt = "2026-07-29T11:00:00.000Z";
	await fixture.persist();
	await assert.rejects(fixture.verify(), /expired/u);

	fixture.manifest.expiresAt = "2026-08-20T11:00:00.000Z";
	fixture.deploymentAttestation.release = "b".repeat(40);
	await fixture.persist();
	await assert.rejects(fixture.verify(), /deployed release does not match/u);
});

test("rejects stale reports and non-canonical timestamps", async (t) => {
	const fixture = await evidenceFixture(t);
	fixture.manifest.reports.find(
		({ category }) => category === "acceptance-alpha",
	).signedAt = "2026-07-20T09:00:00.000Z";
	await fixture.persist();
	await assert.rejects(fixture.verify(), /older than its freshness policy/u);

	fixture.manifest.reports.find(
		({ category }) => category === "acceptance-alpha",
	).signedAt = "2026-07-29T09:00:00.000Z";
	fixture.manifest.issuedAt = "2026-07-29T10:00:00Z";
	await fixture.persist();
	await assert.rejects(fixture.verify(), /canonical UTC ISO timestamp/u);
});

test("requires every signed report to bind release and environment", async (t) => {
	const fixture = await evidenceFixture(t);
	fixture.manifest.reports[0].release = "b".repeat(40);
	await fixture.persist();
	await assert.rejects(fixture.verify(), /report release mismatch/u);

	fixture.manifest.reports[0].release = RELEASE;
	fixture.manifest.reports[0].environment = "production";
	await fixture.persist();
	await assert.rejects(fixture.verify(), /report environment mismatch/u);
});

test("rejects symlinked evidence artifacts", async (t) => {
	const fixture = await evidenceFixture(t);
	const outside = path.join(
		fixture.root,
		"..",
		`outside-${path.basename(fixture.root)}.txt`,
	);
	t.after(() => rm(outside, { force: true }));
	await writeFile(outside, "outside");
	const report = fixture.manifest.reports[0];
	const reportPath = path.join(fixture.root, report.path);
	await rm(reportPath);
	await symlink(outside, reportPath);
	report.sha256 = sha256("outside");
	await fixture.persist();
	await assert.rejects(fixture.verify(), /regular file/u);
});

test("requires fresh control-plane deployment attestation", async (t) => {
	const fixture = await evidenceFixture(t);
	fixture.deploymentAttestation.audience = "operator-input";
	await fixture.persist();
	await assert.rejects(fixture.verify(), /audience is invalid/u);

	fixture.deploymentAttestation.audience = "nvbes-account-release-gate";
	fixture.deploymentAttestation.components[0].observedAt =
		"2026-07-28T10:00:00.000Z";
	await fixture.persist();
	await assert.rejects(fixture.verify(), /older than 24 hours/u);
});

test("production release gate requires signed evidence and production smoke", async () => {
	const releaseGate = await readFile("scripts/release-gate.sh", "utf8");
	const packageJson = JSON.parse(await readFile("package.json", "utf8"));
	for (const variable of [
		"ACCOUNT_ACCEPTANCE_EVIDENCE_FILE",
		"ACCOUNT_ACCEPTANCE_EVIDENCE_SIGNATURE_FILE",
		"ACCOUNT_ACCEPTANCE_TRUSTED_KEY_ID",
		"ACCOUNT_ACCEPTANCE_TRUSTED_PUBLIC_KEY_FILE",
		"ACCOUNT_ACCEPTANCE_TRUSTED_PUBLIC_KEY_SHA256",
		"ACCOUNT_DEPLOYMENT_EXPECTED_COMPONENT_UIDS",
		"ACCOUNT_DEPLOYMENT_TRUSTED_KEY_ID",
		"ACCOUNT_DEPLOYMENT_TRUSTED_PUBLIC_KEY_FILE",
		"ACCOUNT_DEPLOYMENT_TRUSTED_PUBLIC_KEY_SHA256",
		"NVBES_RELEASE_SHA",
		"NVBES_PRODUCTION_WEB_BASE_URL",
		"NVBES_PRODUCTION_API_BASE_URL",
		"NVBES_PRODUCTION_ALLOWED_ORIGINS",
	]) {
		assert.match(releaseGate, new RegExp(`require_env ${variable}\\b`, "u"));
	}
	assert.match(releaseGate, /pnpm check:account-acceptance-evidence/u);
	assert.match(releaseGate, /pnpm check:account-production-target/u);
	assert.match(releaseGate, /pnpm check:account-release-readiness/u);
	assert.match(releaseGate, /NVBES_SMOKE_FORBID_REDIRECTS=1/u);
	assert.match(
		releaseGate,
		/NVBES_WEB_BASE_URL="\$NVBES_PRODUCTION_WEB_BASE_URL"[\s\S]+pnpm test:smoke/u,
	);
	assert.doesNotMatch(releaseGate, /Production URLs are not set/u);
	assert.equal(
		packageJson.scripts["check:account-acceptance-evidence"],
		"node tools/account-quality/verify-acceptance-evidence.mjs",
	);
	assert.equal(
		packageJson.scripts["check:account-release-readiness"],
		"node tools/account-quality/check-account-release-readiness.mjs",
	);
	assert.equal(
		packageJson.scripts["check:account-production-target"],
		"node tools/account-quality/check-production-account-target.mjs",
	);
});

async function evidenceFixture(t) {
	const root = await mkdtemp(path.join(tmpdir(), "account-acceptance-"));
	t.after(() => rm(root, { force: true, recursive: true }));
	await mkdir(path.join(root, "reports"));
	const { privateKey, publicKey } = generateKeyPairSync("ed25519");
	const publicKeyPath = path.join(root, "trusted-public-key.pem");
	const evidencePath = path.join(root, "acceptance-evidence.json");
	const signaturePath = path.join(root, "acceptance-evidence.sig");
	const publicKeyPem = publicKey.export({ format: "pem", type: "spki" });
	await writeFile(publicKeyPath, publicKeyPem);
	const deploymentKeys = generateKeyPairSync("ed25519");
	const deploymentPublicKeyPath = path.join(root, "deployment-public-key.pem");
	const deploymentPublicKeyPem = deploymentKeys.publicKey.export({
		format: "pem",
		type: "spki",
	});
	await writeFile(deploymentPublicKeyPath, deploymentPublicKeyPem);

	const reports = [];
	for (const category of REQUIRED_HUMAN_CATEGORIES) {
		const relativePath = `reports/${category}.txt`;
		const content = `signed ${category} result`;
		await writeFile(path.join(root, relativePath), content);
		reports.push({
			category,
			decision: "passed",
			environment: "staging",
			independent: category === "penetration" ? true : undefined,
			path: relativePath,
			release: RELEASE,
			sha256: sha256(content),
			signedAt: "2026-07-29T09:00:00.000Z",
			signedBy: `${category}-owner`,
		});
	}
	const expectedComponentUids = {
		"account-service": "production/account-service/deployment-123",
		"account-web": "production/account-web/deployment-123",
		"account-worker": "production/account-worker/deployment-123",
	};
	const deploymentAttestation = {
		audience: "nvbes-account-release-gate",
		components: Object.entries(expectedComponentUids).map(
			([name, deploymentUid]) => ({
				deploymentUid,
				desiredReplicas: 4,
				imageDigest: `sha256:${"c".repeat(64)}`,
				name,
				observedAt: "2026-07-30T11:30:00.000Z",
				readyReplicas: 4,
				release: RELEASE,
			}),
		),
		environment: "production",
		issuer: "nvbes-production-control-plane",
		release: RELEASE,
		schemaVersion: 1,
		signerKeyId: "production-control-plane-2026",
	};
	const manifest = {
		deployment: {
			path: "deployment.json",
			sha256: "",
			signaturePath: "deployment.sig",
			signerKeyId: "production-control-plane-2026",
		},
		environment: "staging",
		expiresAt: "2026-08-20T11:00:00.000Z",
		issuedAt: "2026-07-29T10:00:00.000Z",
		release: RELEASE,
		reports,
		schemaVersion: 1,
		signerKeyId: "account-release-board-2026",
	};

	const fixture = {
		deploymentAttestation,
		deploymentTrust: {
			expectedComponentUids,
			expectedPublicKeySha256: sha256(deploymentPublicKeyPem),
			expectedSignerKeyId: "production-control-plane-2026",
			publicKeyPath: deploymentPublicKeyPath,
		},
		evidencePath,
		manifest,
		publicKeyPath,
		root,
		signaturePath,
		async persist() {
			const deploymentContent = Buffer.from(
				`${JSON.stringify(deploymentAttestation, null, 2)}\n`,
			);
			await writeFile(
				path.join(root, manifest.deployment.path),
				deploymentContent,
			);
			await writeFile(
				path.join(root, manifest.deployment.signaturePath),
				sign(null, deploymentContent, deploymentKeys.privateKey),
			);
			manifest.deployment.sha256 = sha256(deploymentContent);
			const content = Buffer.from(`${JSON.stringify(manifest, null, 2)}\n`);
			await writeFile(evidencePath, content);
			await writeFile(signaturePath, sign(null, content, privateKey));
		},
		async verify(overrides = {}) {
			return verifyAcceptanceEvidence({
				deploymentTrust: fixture.deploymentTrust,
				evidencePath,
				expectedPublicKeySha256: sha256(publicKeyPem),
				expectedRelease: RELEASE,
				expectedSignerKeyId: "account-release-board-2026",
				now: NOW,
				publicKeyPath,
				signaturePath,
				...overrides,
			});
		},
	};
	await fixture.persist();
	return fixture;
}

function sha256(content) {
	return createHash("sha256").update(content).digest("hex");
}
