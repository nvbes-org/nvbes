import assert from 'node:assert/strict';
import { createHash, createPublicKey, verify } from 'node:crypto';
import { readFileSync, realpathSync, statSync } from 'node:fs';
import path from 'node:path';
import { verifyTypescriptMeasurements } from './v1-typescript-measurements.mjs';
import { verifyRustMeasurements } from './v1-rust-measurements.mjs';

export const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex');

export function confinedRead(directory, relative) {
  assert(typeof relative === 'string' && relative.length > 0 && !path.isAbsolute(relative));
  assert(!relative.split(/[\\/]/u).includes('..'), 'Parent traversal forbidden');
  const root = realpathSync(directory);
  const location = realpathSync(path.resolve(root, relative));
  assert(location.startsWith(`${root}${path.sep}`), 'Artifact escapes evidence directory');
  const stat = statSync(location);
  assert(stat.isFile() && stat.size <= 16 * 1024 * 1024, 'Invalid or oversized evidence file');
  return readFileSync(location);
}

export function verifyBundle({
  bytes,
  signature,
  publicKey,
  publicKeyDigest,
  artifactDirectory,
  repository,
  workflows,
  getRun,
  getCiArtifact,
  units,
}) {
  assert(Array.isArray(units) && units.length > 0, 'Candidate measurement scope required');
  assert(/^[a-f0-9]{64}$/u.test(publicKeyDigest ?? ''), 'Pinned public key digest required');
  assert.equal(sha256(publicKey), publicKeyDigest, 'Untrusted signing key');
  const key = createPublicKey(publicKey);
  assert.equal(key.asymmetricKeyType, 'ed25519', 'Ed25519 evidence key required');
  assert(verify(null, bytes, key, signature), 'Invalid evidence signature');
  const bundle = JSON.parse(bytes.toString('utf8'));
  assert(bundle.incomplete !== true, 'Incomplete evidence draft cannot be released');
  assert(Array.isArray(bundle.results) && bundle.results.length > 0, 'Missing results');
  assert(
    Array.isArray(bundle.artifacts) && bundle.artifacts.length > 0,
    'Missing bundle artifacts',
  );
  const artifacts = new Map();
  let artifactBytesTotal = 0;
  for (const artifact of bundle.artifacts) {
    assert(artifacts.size < 256, 'Too many evidence artifacts');
    assert(!artifacts.has(artifact.path), 'Duplicate artifact');
    const artifactBytes = confinedRead(artifactDirectory, artifact.path);
    artifactBytesTotal += artifactBytes.length;
    assert(artifactBytesTotal <= 64 * 1024 * 1024, 'Evidence artifact memory budget exceeded');
    assert.equal(sha256(artifactBytes), artifact.sha256, 'Artifact digest mismatch');
    artifacts.set(artifact.path, artifactBytes);
  }
  const runs = new Map();
  for (const result of [...bundle.results, ...(bundle.measurements ?? [])]) {
    assert(
      Array.isArray(result.artifacts) &&
        result.artifacts.length > 0 &&
        result.artifacts.every((file) => artifacts.has(file)),
      'Unverified suite artifact',
    );
    const producer = result.producer;
    assert(producer && ['github-actions', 'operator'].includes(producer.kind), 'Unknown producer');
    if (producer.kind === 'operator') {
      assert(!Object.hasOwn(result, 'unit'), 'Measurements require a CI producer');
      assert(
        result.artifacts.every((file) => !bundle.artifacts.find((entry) => entry.path === file).ci),
        'Operator evidence must remain separate from CI artifacts',
      );
      assert(
        typeof producer.signedBy === 'string' && producer.signedBy.trim(),
        'Missing operator identity',
      );
      assert.equal(producer.publicKeyDigest, publicKeyDigest, 'Operator identity key mismatch');
      continue;
    }
    assert.equal(producer.repository, repository, 'Wrong producer repository');
    assert(/^[1-9][0-9]*$/u.test(String(producer.runId)), 'Invalid workflow run ID');
    assert(workflows.includes(producer.workflow), 'Untrusted workflow');
    const id = String(producer.runId);
    assert(Number.isSafeInteger(Number(id)), 'Invalid workflow run ID');
    if (!runs.has(id)) {
      assert(runs.size < 32, 'Too many evidence runs');
      runs.set(id, getRun(producer.runId));
    }
    const run = runs.get(id);
    assert.equal(String(run.id), id, 'Wrong workflow run ID');
    assert(
      Number.isSafeInteger(run.repository.id) && run.repository.id > 0,
      'Missing repository ID',
    );
    assert.equal(run.repository.full_name, repository);
    assert.equal(run.head_sha, bundle.sha, 'Workflow SHA mismatch');
    assert.equal(run.path, producer.workflow, 'Workflow path mismatch');
    assert.equal(run.status, 'completed');
    assert.equal(run.conclusion, 'success');
    assert.equal(run.run_attempt, 1, 'Rerun cannot replace failed evidence');
    assert(['push', 'workflow_dispatch'].includes(run.event), 'Untrusted workflow event');
    for (const file of result.artifacts) {
      const reference = bundle.artifacts.find((entry) => entry.path === file).ci;
      assert(reference && typeof getCiArtifact === 'function', 'Missing CI artifact provenance');
      const published = getCiArtifact({ reference, run, sha: bundle.sha });
      assert(
        Buffer.isBuffer(published) && published.equals(artifacts.get(file)),
        'Local evidence differs from published CI artifact',
      );
    }
  }
  verifyTypescriptMeasurements(bundle, units, artifacts);
  verifyRustMeasurements(bundle, units, artifacts);
  return bundle;
}
