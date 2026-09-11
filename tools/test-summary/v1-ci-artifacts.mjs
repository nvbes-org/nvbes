import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';

const LIMIT = 16 * 1024 * 1024;
const digest = (bytes) => `sha256:${createHash('sha256').update(bytes).digest('hex')}`;
export function artifactId(value) {
  assert(
    /^[1-9][0-9]*$/u.test(String(value)) && Number.isSafeInteger(Number(value)),
    'Invalid CI artifact ID',
  );
  return String(value);
}

export function validateArtifact(metadata, reference, run, sha, now = Date.now()) {
  assert.equal(String(metadata.id), artifactId(reference.artifactId), 'Wrong CI artifact ID');
  assert.equal(metadata.workflow_run?.id, run.id, 'Artifact belongs to another run');
  assert.equal(metadata.workflow_run?.head_sha, sha, 'Artifact belongs to another SHA');
  assert.equal(
    metadata.workflow_run?.repository_id,
    run.repository.id,
    'Artifact repository mismatch',
  );
  assert.equal(
    metadata.workflow_run?.head_repository_id,
    run.repository.id,
    'Fork artifact forbidden',
  );
  assert(
    metadata.expired === false && Date.parse(metadata.expires_at) > now,
    'Expired CI artifact',
  );
  assert(
    Number.isFinite(Date.parse(metadata.created_at)) && Date.parse(metadata.created_at) <= now,
    'Invalid CI artifact creation date',
  );
  assert(/^sha256:[a-f0-9]{64}$/u.test(metadata.digest ?? ''), 'Missing GitHub artifact digest');
  assert(
    Number.isSafeInteger(metadata.size_in_bytes) &&
      metadata.size_in_bytes > 0 &&
      metadata.size_in_bytes <= LIMIT,
    'Oversized CI artifact',
  );
}

// Never extract archive paths onto the filesystem. unzip streams one exact,
// non-glob member; duplicate members and unsafe names are rejected first.
export function readArchiveMember(bytes, member) {
  assert(Buffer.isBuffer(bytes) && bytes.length <= LIMIT, 'Oversized CI archive');
  assert(
    typeof member === 'string' &&
      /^[a-zA-Z0-9_][a-zA-Z0-9_./-]*$/u.test(member) &&
      !member.split('/').some((part) => part === '..' || part === '.' || part === ''),
    'Unsafe CI archive member',
  );
  const directory = mkdtempSync(path.join(os.tmpdir(), 'nvbes-ci-artifact-'));
  try {
    const archive = path.join(directory, 'artifact.zip');
    writeFileSync(archive, bytes, { mode: 0o600 });
    const names = execFileSync('unzip', ['-Z1', archive], {
      encoding: 'utf8',
      timeout: 10000,
      maxBuffer: 1024 * 1024,
    })
      .trimEnd()
      .split('\n');
    assert.equal(
      names.filter((name) => name === member).length,
      1,
      'Missing or duplicate archive member',
    );
    return execFileSync('unzip', ['-p', archive, member], { timeout: 10000, maxBuffer: LIMIT });
  } catch {
    throw new Error('Invalid, missing, duplicate or oversized CI archive member');
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
}

function githubApi(endpoint) {
  try {
    return execFileSync('gh', ['api', '--hostname', 'github.com', endpoint], {
      timeout: 30000,
      maxBuffer: LIMIT,
      stdio: ['ignore', 'pipe', 'pipe'],
    });
  } catch {
    throw new Error('Unable to retrieve CI artifact from GitHub');
  }
}

export function githubArtifactReader(repository, { api = githubApi } = {}) {
  assert(/^[a-zA-Z0-9_.-]+\/[a-zA-Z0-9_.-]+$/u.test(repository), 'Invalid repository');
  const archives = new Map();
  let downloaded = 0;
  return ({ reference, run, sha }) => {
    const id = artifactId(reference.artifactId);
    if (!archives.has(id)) {
      assert(archives.size < 32, 'Too many CI archives');
      const metadata = JSON.parse(api(`repos/${repository}/actions/artifacts/${id}`));
      validateArtifact(metadata, reference, run, sha);
      assert(
        downloaded + metadata.size_in_bytes <= 64 * 1024 * 1024,
        'CI download budget exceeded',
      );
      const bytes = api(`repos/${repository}/actions/artifacts/${id}/zip`);
      assert.equal(bytes.length, metadata.size_in_bytes, 'CI archive size mismatch');
      assert.equal(digest(bytes), metadata.digest, 'GitHub archive digest mismatch');
      downloaded += bytes.length;
      archives.set(id, { metadata, bytes });
    }
    const { metadata, bytes } = archives.get(id);
    validateArtifact(metadata, reference, run, sha);
    return readArchiveMember(bytes, reference.member);
  };
}
