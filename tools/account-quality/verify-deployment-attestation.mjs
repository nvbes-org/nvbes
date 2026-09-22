import { createHash, verify } from 'node:crypto';
import { constants } from 'node:fs';
import { open, realpath } from 'node:fs/promises';
import path from 'node:path';

const REQUIRED_COMPONENTS = ['account-service', 'account-web', 'account-worker'];
const SHA256_PATTERN = /^[a-f0-9]{64}$/u;

export async function verifyDeploymentAttestation({
  deployment,
  evidenceRoot,
  expectedRelease,
  trust,
  now,
}) {
  assert(isRecord(deployment), 'production deployment evidence is required');
  assert(isRecord(trust), 'deployment attestation trust configuration is required');
  assert(
    SHA256_PATTERN.test(trust.expectedPublicKeySha256 ?? ''),
    'deployment trusted public key SHA-256 is required',
  );
  assert(nonEmpty(trust.expectedSignerKeyId), 'deployment trusted signer key id is required');
  validateExpectedComponentUids(trust.expectedComponentUids);
  assert(
    deployment.signerKeyId === trust.expectedSignerKeyId,
    'deployment attestation signer key id is not trusted',
  );

  const attestationPath = await verifyArtifact(
    deployment,
    evidenceRoot,
    'production deployment attestation',
  );
  const signaturePath = await resolveEvidenceFile(
    deployment.signaturePath,
    evidenceRoot,
    4096,
    'production deployment signature',
  );
  const [attestationBytes, signature, publicKey] = await Promise.all([
    readBoundedRegularFile(
      attestationPath,
      100 * 1024 * 1024,
      'production deployment attestation',
    ).then((entry) => entry.content),
    readBoundedRegularFile(signaturePath, 4096, 'production deployment signature').then(
      (entry) => entry.content,
    ),
    readBoundedRegularFile(trust.publicKeyPath, 16_384, 'deployment trusted public key'),
  ]);
  assert(
    sha256Bytes(publicKey.content) === trust.expectedPublicKeySha256,
    'deployment trusted public key fingerprint mismatch',
  );
  assert(
    verify(null, attestationBytes, publicKey.content, signature),
    'production deployment attestation signature is invalid',
  );

  const attestation = parseAttestation(attestationBytes);
  assert(attestation.schemaVersion === 1, 'unsupported deployment attestation schema');
  assert(
    attestation.signerKeyId === trust.expectedSignerKeyId,
    'deployment attestation key id mismatch',
  );
  assert(
    attestation.issuer === 'nvbes-production-control-plane',
    'deployment attestation issuer is invalid',
  );
  assert(
    attestation.audience === 'nvbes-account-release-gate',
    'deployment attestation audience is invalid',
  );
  assert(attestation.environment === 'production', 'deployment attestation must target production');
  assert(
    attestation.release === expectedRelease,
    'deployed release does not match acceptance release',
  );
  validateComponents(attestation.components, trust.expectedComponentUids, expectedRelease, now);
}

function validateComponents(components, expectedUids, expectedRelease, now) {
  assert(
    Array.isArray(components) && components.length === REQUIRED_COMPONENTS.length,
    'deployment attestation must contain exactly the Account components',
  );
  const observed = new Set();
  for (const component of components) {
    assert(isRecord(component), 'deployment component must be an object');
    assert(
      REQUIRED_COMPONENTS.includes(component.name),
      `unexpected deployment component ${String(component.name)}`,
    );
    assert(!observed.has(component.name), `duplicate deployment component ${component.name}`);
    observed.add(component.name);
    assert(
      component.deploymentUid === expectedUids[component.name],
      `${component.name} deployment UID is not trusted`,
    );
    assert(component.release === expectedRelease, `${component.name} release mismatch`);
    assert(
      /^sha256:[a-f0-9]{64}$/u.test(component.imageDigest ?? ''),
      `${component.name} image digest is invalid`,
    );
    assert(
      Number.isSafeInteger(component.desiredReplicas) && component.desiredReplicas > 0,
      `${component.name} desired replicas are invalid`,
    );
    assert(
      component.readyReplicas === component.desiredReplicas,
      `${component.name} does not show every replica ready`,
    );
    const observedAt = timestamp(component.observedAt, `${component.name}.observedAt`);
    assert(observedAt <= now.getTime() + 5 * 60 * 1000, `${component.name} is future-dated`);
    assert(
      now.getTime() - observedAt <= 24 * 60 * 60 * 1000,
      `${component.name} evidence is older than 24 hours`,
    );
  }
}

function validateExpectedComponentUids(expectedUids) {
  assert(isRecord(expectedUids), 'trusted deployment component UIDs are required');
  assert(
    Object.keys(expectedUids)
      .sort((a, b) => a.localeCompare(b))
      .join(',') === REQUIRED_COMPONENTS.join(','),
    'trusted deployment component UID set is invalid',
  );
  for (const name of REQUIRED_COMPONENTS) {
    assert(
      typeof expectedUids[name] === 'string' &&
        /^[a-zA-Z0-9][a-zA-Z0-9:._/-]{2,255}$/u.test(expectedUids[name]),
      `${name} trusted deployment UID is invalid`,
    );
  }
}

async function verifyArtifact(entry, evidenceRoot, label) {
  assert(nonEmpty(entry.path), `${label} path is required`);
  assert(SHA256_PATTERN.test(entry.sha256 ?? ''), `${label} SHA-256 is invalid`);
  const artifactPath = await resolveEvidenceFile(
    entry.path,
    evidenceRoot,
    100 * 1024 * 1024,
    label,
  );
  assert((await sha256File(artifactPath)) === entry.sha256, `${label} digest mismatch`);
  return artifactPath;
}

async function resolveEvidenceFile(relativePath, evidenceRoot, maxBytes, label) {
  assert(nonEmpty(relativePath), `${label} path is required`);
  const candidate = path.resolve(evidenceRoot, relativePath);
  assert(
    candidate.startsWith(`${evidenceRoot}${path.sep}`),
    `${label} path escapes the evidence directory`,
  );
  // Open without following symlinks, then validate the opened file itself:
  // check-then-use happens on the same file descriptor, so a concurrent
  // swap between the check and the read cannot redirect the read.
  const handle = await open(candidate, constants.O_RDONLY | constants.O_NOFOLLOW);
  try {
    const metadata = await handle.stat();
    assert(metadata.isFile(), `${label} must be a regular file`);
    assert(metadata.size > 0 && metadata.size <= maxBytes, `${label} has an invalid size`);
  } finally {
    await handle.close();
  }
  const artifactPath = await realpath(candidate);
  assert(
    artifactPath.startsWith(`${evidenceRoot}${path.sep}`),
    `${label} resolves outside the evidence directory`,
  );
  return artifactPath;
}

async function readBoundedRegularFile(filePath, maxBytes, label) {
  assert(nonEmpty(filePath), `${label} path is required`);
  const resolved = path.resolve(filePath);
  // Same-descriptor check-then-read: O_NOFOLLOW rejects symlinks at open
  // time and fstat validates the exact file that is subsequently read.
  const handle = await open(resolved, constants.O_RDONLY | constants.O_NOFOLLOW);
  try {
    const metadata = await handle.stat();
    assert(metadata.isFile(), `${label} must be a regular file`);
    assert(metadata.size > 0 && metadata.size <= maxBytes, `${label} has an invalid size`);
    const content = await handle.readFile();
    assert(content.length > 0 && content.length <= maxBytes, `${label} has an invalid size`);
    return { content };
  } finally {
    await handle.close();
  }
}

async function sha256File(filePath) {
  const handle = await open(filePath, constants.O_RDONLY | constants.O_NOFOLLOW);
  try {
    const metadata = await handle.stat();
    assert(metadata.isFile(), 'attestation artifact must be a regular file');
    const hash = createHash('sha256');
    for await (const chunk of handle.createReadStream()) {
      hash.update(chunk);
    }
    return hash.digest('hex');
  } finally {
    await handle.close();
  }
}

function parseAttestation(content) {
  try {
    const attestation = JSON.parse(content.toString('utf8'));
    assert(isRecord(attestation), 'production deployment attestation must be an object');
    return attestation;
  } catch (error) {
    if (error instanceof SyntaxError) {
      throw new Error('production deployment attestation is not valid JSON');
    }
    throw error;
  }
}

function sha256Bytes(content) {
  return createHash('sha256').update(content).digest('hex');
}

function timestamp(value, label) {
  assert(typeof value === 'string', `${label} must be an ISO timestamp`);
  const parsed = Date.parse(value);
  assert(Number.isFinite(parsed), `${label} must be an ISO timestamp`);
  return parsed;
}

function nonEmpty(value) {
  return typeof value === 'string' && value.trim().length > 0 && value.length <= 256;
}

function isRecord(value) {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}
