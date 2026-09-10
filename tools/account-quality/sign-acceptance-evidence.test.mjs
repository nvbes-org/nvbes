import assert from 'node:assert/strict';
import { createHash, sign } from 'node:crypto';
import { chmod, mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import test from 'node:test';

import {
  REQUIRED_HUMAN_CATEGORIES,
  verifyAcceptanceEvidence,
} from './verify-acceptance-evidence.mjs';
import { generateAcceptanceKey, signAcceptanceEvidence } from './sign-acceptance-evidence.mjs';

const RELEASE = 'a'.repeat(40);
const NOW = new Date('2026-07-30T12:00:00.000Z');

test('signs an evidence packet that the release gate accepts', async (t) => {
  const fixture = await evidenceFixture(t);
  const publicKeyFixture = await keypairFixture(t);
  const publicKeyFingerprint = createHash('sha256')
    .update(await readFile(publicKeyFixture.publicPath))
    .digest('hex');

  const signed = await signAcceptanceEvidence({
    deploymentKeyId: 'production-control-plane-2026',
    evidenceRoot: fixture.root,
    now: NOW,
    privateKeyPath: publicKeyFixture.privatePath,
    publicKeyPath: publicKeyFixture.publicPath,
    release: RELEASE,
    reports: fixture.reports,
    signerKeyId: 'account-release-board-2026',
  });
  assert.equal(signed.publicKeySha256, publicKeyFingerprint);
  assert.equal(signed.release, RELEASE);
  assert.equal(signed.signerKeyId, 'account-release-board-2026');

  const result = await fixture.verify({
    evidencePath: signed.manifestPath,
    publicKeyPath: publicKeyFixture.publicPath,
    signaturePath: signed.signaturePath,
    expectedPublicKeySha256: publicKeyFingerprint,
  });
  assert.deepEqual(result, {
    categories: REQUIRED_HUMAN_CATEGORIES.length,
    deploymentRelease: RELEASE,
    release: RELEASE,
    signerKeyId: 'account-release-board-2026',
  });
});

test('rejects every tampered packet via the released verifier', async (t) => {
  const fixture = await evidenceFixture(t);
  const keypair = await keypairFixture(t);
  const signed = await signAcceptanceEvidence({
    deploymentKeyId: 'production-control-plane-2026',
    evidenceRoot: fixture.root,
    now: NOW,
    privateKeyPath: keypair.privatePath,
    release: RELEASE,
    reports: fixture.reports,
    signerKeyId: 'account-release-board-2026',
  });

  await writeFile(path.join(fixture.root, fixture.manifest.reports[0].path), 'tampered');
  await assert.rejects(
    fixture.verify({
      evidencePath: signed.manifestPath,
      publicKeyPath: keypair.publicPath,
      signaturePath: signed.signaturePath,
      expectedPublicKeySha256: keypair.fingerprint,
    }),
    /digest mismatch/u,
  );

  const manifest = JSON.parse(await readFile(signed.manifestPath, 'utf8'));
  manifest.release = 'b'.repeat(40);
  await writeFile(signed.manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
  await assert.rejects(
    fixture.verify({
      evidencePath: signed.manifestPath,
      publicKeyPath: keypair.publicPath,
      signaturePath: signed.signaturePath,
      expectedPublicKeySha256: keypair.fingerprint,
    }),
    /signature is invalid/u,
  );
});

test('refuses to sign when a required report is missing', async (t) => {
  const fixture = await evidenceFixture(t);
  const keypair = await keypairFixture(t);
  await rm(path.join(fixture.root, 'reports/acceptance-alpha.txt'));
  await assert.rejects(
    signAcceptanceEvidence({
      deploymentKeyId: 'production-control-plane-2026',
      evidenceRoot: fixture.root,
      now: NOW,
      privateKeyPath: keypair.privatePath,
      release: RELEASE,
      reports: fixture.reports,
      signerKeyId: 'account-release-board-2026',
    }),
    /must be a regular file/u,
  );
});

test('refuses stale reports before signing', async (t) => {
  const fixture = await evidenceFixture(t);
  const keypair = await keypairFixture(t);
  const reports = fixture.reports.map((report) => ({
    ...report,
    signedAt: report.category === 'acceptance-alpha' ? '2026-07-20T09:00:00.000Z' : report.signedAt,
  }));
  await assert.rejects(
    signAcceptanceEvidence({
      deploymentKeyId: 'production-control-plane-2026',
      evidenceRoot: fixture.root,
      now: NOW,
      privateKeyPath: keypair.privatePath,
      release: RELEASE,
      reports,
      signerKeyId: 'account-release-board-2026',
    }),
    /older than its freshness policy/u,
  );
});

test('refuses non-canonical and future timestamps', async (t) => {
  const fixture = await evidenceFixture(t);
  const keypair = await keypairFixture(t);
  await assert.rejects(
    signAcceptanceEvidence({
      deploymentKeyId: 'production-control-plane-2026',
      evidenceRoot: fixture.root,
      expiresAt: '2026-08-20T11:00:00Z',
      issuedAt: '2026-07-29T10:00:00Z',
      now: NOW,
      privateKeyPath: keypair.privatePath,
      release: RELEASE,
      reports: fixture.reports,
      signerKeyId: 'account-release-board-2026',
    }),
    /canonical UTC ISO timestamp/u,
  );
  await assert.rejects(
    signAcceptanceEvidence({
      deploymentKeyId: 'production-control-plane-2026',
      evidenceRoot: fixture.root,
      issuedAt: '2026-08-01T12:00:00.000Z',
      now: NOW,
      privateKeyPath: keypair.privatePath,
      release: RELEASE,
      reports: fixture.reports,
      signerKeyId: 'account-release-board-2026',
    }),
    /future-dated/u,
  );
});

test('refuses invalid release, key ids and environment', async (t) => {
  const fixture = await evidenceFixture(t);
  const keypair = await keypairFixture(t);
  for (const overrides of [
    { release: 'not-a-sha' },
    { signerKeyId: 'AccountBoard2026' },
    { deploymentKeyId: 'UPPERcase' },
    { environment: 'production' },
  ]) {
    await assert.rejects(
      signAcceptanceEvidence({
        deploymentKeyId: 'production-control-plane-2026',
        evidenceRoot: fixture.root,
        now: NOW,
        privateKeyPath: keypair.privatePath,
        release: RELEASE,
        reports: fixture.reports,
        signerKeyId: 'account-release-board-2026',
        ...overrides,
      }),
      /expected|invalid|must target staging/u,
    );
  }
});

test('rejects a world-readable private key', async (t) => {
  const evidence = await evidenceFixture(t);
  const keypair = await keypairFixture(t);
  await chmod(keypair.privatePath, 0o644);
  await assert.rejects(
    signAcceptanceEvidence({
      deploymentKeyId: 'production-control-plane-2026',
      evidenceRoot: evidence.root,
      now: NOW,
      privateKeyPath: keypair.privatePath,
      release: RELEASE,
      reports: evidence.reports,
      signerKeyId: 'account-release-board-2026',
    }),
    /must have permissions 0600/u,
  );
});

test('generates a key pair and refuses to overwrite it', async (t) => {
  const directory = await mkdtemp(path.join(tmpdir(), 'account-keys-'));
  t.after(() => rm(directory, { force: true, recursive: true }));
  const first = await generateAcceptanceKey({ directory });
  assert.match(await readFile(first.privateFile, 'utf8'), /PRIVATE KEY/u);
  assert.match(await readFile(first.publicFile, 'utf8'), /PUBLIC KEY/u);
  assert.match(first.publicKeySha256, /^[a-f0-9]{64}$/u);
  const privateMode = (await readFile(first.privateFile)).length > 0;
  assert.equal(privateMode, true);
  await assert.rejects(generateAcceptanceKey({ directory }), /EEXIST|already exists/u);
});

async function evidenceFixture(t) {
  const root = await mkdtemp(path.join(tmpdir(), 'account-evidence-'));
  t.after(() => rm(root, { force: true, recursive: true }));
  await mkdir(path.join(root, 'reports'));

  const reports = REQUIRED_HUMAN_CATEGORIES.map((category) => {
    const relativePath = `reports/${category}.txt`;
    return {
      category,
      path: relativePath,
      signedBy: `${category}-owner`,
    };
  });
  for (const report of reports) {
    await writeFile(path.join(root, report.path), `signed ${report.category} result`);
  }

  const deploymentKeys = (await import('node:crypto')).generateKeyPairSync('ed25519');
  const deploymentSignerKeyId = 'production-control-plane-2026';
  const expectedComponentUids = {
    'account-service': 'production/account-service/deployment-123',
    'account-web': 'production/account-web/deployment-123',
    'account-worker': 'production/account-worker/deployment-123',
  };
  const deploymentAttestation = {
    audience: 'nvbes-account-release-gate',
    components: Object.entries(expectedComponentUids).map(([name, deploymentUid]) => ({
      deploymentUid,
      desiredReplicas: 4,
      imageDigest: `sha256:${'c'.repeat(64)}`,
      name,
      observedAt: '2026-07-30T11:30:00.000Z',
      readyReplicas: 4,
      release: RELEASE,
    })),
    environment: 'production',
    issuer: 'nvbes-production-control-plane',
    release: RELEASE,
    schemaVersion: 1,
    signerKeyId: deploymentSignerKeyId,
  };
  const deploymentContent = Buffer.from(`${JSON.stringify(deploymentAttestation, null, 2)}\n`);
  await writeFile(path.join(root, 'deployment.json'), deploymentContent);
  await writeFile(
    path.join(root, 'deployment.sig'),
    sign(null, deploymentContent, deploymentKeys.privateKey),
  );
  const deploymentPublicKeyPem = deploymentKeys.publicKey.export({
    format: 'pem',
    type: 'spki',
  });

  const manifest = {
    deployment: {
      path: 'deployment.json',
      sha256: createHash('sha256').update(deploymentContent).digest('hex'),
      signaturePath: 'deployment.sig',
      signerKeyId: deploymentSignerKeyId,
    },
    environment: 'staging',
    expiresAt: '2026-08-20T11:00:00.000Z',
    issuedAt: '2026-07-29T10:00:00.000Z',
    release: RELEASE,
    reports: REQUIRED_HUMAN_CATEGORIES.map((category) => ({
      category,
      decision: 'passed',
      environment: 'staging',
      independent: category === 'penetration' ? true : undefined,
      path: `reports/${category}.txt`,
      release: RELEASE,
      signedAt: '2026-07-29T09:00:00.000Z',
      signedBy: `${category}-owner`,
    })),
    schemaVersion: 1,
    signerKeyId: 'account-release-board-2026',
  };

  return {
    deploymentTrust: {
      expectedComponentUids,
      expectedPublicKeySha256: createHash('sha256').update(deploymentPublicKeyPem).digest('hex'),
      expectedSignerKeyId: deploymentSignerKeyId,
      publicKeyPath: path.join(root, 'deployment-public-key.pem'),
    },
    deploymentKeys,
    manifest,
    reports,
    root,
    async verify({ evidencePath, publicKeyPath, signaturePath, expectedPublicKeySha256 }) {
      await writeFile(path.join(root, 'deployment-public-key.pem'), deploymentPublicKeyPem);
      return verifyAcceptanceEvidence({
        deploymentTrust: this.deploymentTrust,
        evidencePath,
        expectedPublicKeySha256,
        expectedRelease: RELEASE,
        expectedSignerKeyId: 'account-release-board-2026',
        now: NOW,
        publicKeyPath,
        signaturePath,
      });
    },
  };
}

async function keypairFixture(t) {
  const directory = await mkdtemp(path.join(tmpdir(), 'account-keys-'));
  t.after(() => rm(directory, { force: true, recursive: true }));
  const result = await generateAcceptanceKey({ directory, name: 'release-board' });
  return {
    fingerprint: result.publicKeySha256,
    privatePath: result.privateFile,
    publicPath: result.publicFile,
  };
}
