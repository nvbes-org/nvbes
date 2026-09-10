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
  units,
}) {
  assert(Array.isArray(units) && units.length > 0, 'Candidate measurement scope required');
  assert(/^[a-f0-9]{64}$/u.test(publicKeyDigest ?? ''), 'Pinned public key digest required');
  assert.equal(sha256(publicKey), publicKeyDigest, 'Untrusted signing key');
  const key = createPublicKey(publicKey);
  assert.equal(key.asymmetricKeyType, 'ed25519', 'Ed25519 evidence key required');
  assert(verify(null, bytes, key, signature), 'Invalid evidence signature');
  const bundle = JSON.parse(bytes.toString('utf8'));
  assert(Array.isArray(bundle.results) && bundle.results.length > 0, 'Missing results');
  assert(
    Array.isArray(bundle.artifacts) && bundle.artifacts.length > 0,
    'Missing bundle artifacts',
  );
  const artifacts = new Map();
  for (const artifact of bundle.artifacts) {
    assert(!artifacts.has(artifact.path), 'Duplicate artifact');
    const artifactBytes = confinedRead(artifactDirectory, artifact.path);
    assert.equal(sha256(artifactBytes), artifact.sha256, 'Artifact digest mismatch');
    artifacts.set(artifact.path, artifactBytes);
  }
  for (const result of bundle.results) {
    assert(
      Array.isArray(result.artifacts) &&
        result.artifacts.length > 0 &&
        result.artifacts.every((file) => artifacts.has(file)),
      'Unverified suite artifact',
    );
    const producer = result.producer;
    assert(producer && ['github-actions', 'operator'].includes(producer.kind), 'Unknown producer');
    if (producer.kind === 'operator') {
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
    const run = getRun(producer.runId);
    assert.equal(run.repository.full_name, repository);
    assert.equal(run.head_sha, bundle.sha, 'Workflow SHA mismatch');
    assert.equal(run.path, producer.workflow, 'Workflow path mismatch');
    assert.equal(run.status, 'completed');
    assert.equal(run.conclusion, 'success');
    assert.equal(run.run_attempt, 1, 'Rerun cannot replace failed evidence');
    assert(['push', 'workflow_dispatch'].includes(run.event), 'Untrusted workflow event');
  }
  verifyTypescriptMeasurements(bundle, units, artifacts);
  verifyRustMeasurements(bundle, units, artifacts);
  return bundle;
}
