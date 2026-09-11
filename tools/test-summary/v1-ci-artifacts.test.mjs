import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { test } from 'node:test';
import {
  artifactId,
  githubArtifactReader,
  readArchiveMember,
  validateArtifact,
} from './v1-ci-artifacts.mjs';

function fixture() {
  const sha = 'a'.repeat(40);
  const reference = { artifactId: 456, member: 'report.json' };
  const run = { id: 123, repository: { id: 1 } };
  const now = Date.parse('2026-09-10T12:00:00Z');
  const metadata = {
    id: 456,
    workflow_run: { id: 123, head_sha: sha, repository_id: 1, head_repository_id: 1 },
    expired: false,
    expires_at: '2026-09-11T00:00:00Z',
    created_at: '2026-09-10T10:00:00Z',
    digest: `sha256:${'b'.repeat(64)}`,
    size_in_bytes: 100,
  };
  return { metadata, reference, run, sha, now };
}
const verify = (f) => validateArtifact(f.metadata, f.reference, f.run, f.sha, f.now);

test('accepts an unexpired artifact from the exact run and repository', () => {
  assert.doesNotThrow(() => verify(fixture()));
});

for (const [name, change] of [
  [
    'wrong artifact',
    (m) => {
      m.id++;
    },
  ],
  [
    'wrong run',
    (m) => {
      m.workflow_run.id++;
    },
  ],
  [
    'wrong SHA',
    (m) => {
      m.workflow_run.head_sha = 'b'.repeat(40);
    },
  ],
  [
    'wrong repository',
    (m) => {
      m.workflow_run.repository_id++;
    },
  ],
  [
    'fork',
    (m) => {
      m.workflow_run.head_repository_id++;
    },
  ],
  [
    'expired flag',
    (m) => {
      m.expired = true;
    },
  ],
  [
    'expired time',
    (m) => {
      m.expires_at = '2026-09-10T11:00:00Z';
    },
  ],
  [
    'invalid time',
    (m) => {
      m.expires_at = 'unknown';
    },
  ],
  [
    'future creation',
    (m) => {
      m.created_at = '2026-09-11T11:00:00Z';
    },
  ],
  [
    'missing digest',
    (m) => {
      delete m.digest;
    },
  ],
  [
    'wrong digest algorithm',
    (m) => {
      m.digest = `sha1:${'b'.repeat(40)}`;
    },
  ],
  [
    'oversized archive',
    (m) => {
      m.size_in_bytes = 16 * 1024 * 1024 + 1;
    },
  ],
  [
    'invalid size',
    (m) => {
      m.size_in_bytes = -1;
    },
  ],
]) {
  test(`rejects ${String(name)}`, () => {
    const f = fixture();
    change(f.metadata);
    assert.throws(() => verify(f));
  });
}

test('artifact IDs cannot alter API endpoints or lose integer precision', () => {
  assert.equal(artifactId(456), '456');
  for (const value of ['../123', '-1', 0, 1.5, '01', '9007199254740992', undefined])
    assert.throws(() => artifactId(value));
});

test('streams a bounded ZIP member without extracting archive paths', (t) => {
  const directory = mkdtempSync(path.join(os.tmpdir(), 'nvbes-ci-zip-test-'));
  t.after(() => rmSync(directory, { recursive: true, force: true }));
  writeFileSync(path.join(directory, 'report.json'), '{"passed":true}');
  execFileSync('zip', ['-q', 'fixture.zip', 'report.json'], { cwd: directory });
  const bytes = readFileSync(path.join(directory, 'fixture.zip'));
  assert.equal(readArchiveMember(bytes, 'report.json').toString(), '{"passed":true}');
  const end = bytes.subarray(bytes.length - 22);
  const central = bytes.subarray(end.readUInt32LE(16), bytes.length - 22);
  const duplicateEnd = Buffer.from(end);
  duplicateEnd.writeUInt16LE(2, 8);
  duplicateEnd.writeUInt16LE(2, 10);
  duplicateEnd.writeUInt32LE(central.length * 2, 12);
  const duplicated = Buffer.concat([bytes.subarray(0, bytes.length - 22), central, duplicateEnd]);
  assert.throws(() => readArchiveMember(duplicated, 'report.json'), /duplicate/u);
  for (const member of [
    '../report.json',
    '/report.json',
    '*.json',
    '-report.json',
    'a//b',
    'a/./b',
    'absent.json',
  ])
    assert.throws(() => readArchiveMember(bytes, member));
  assert.throws(() => readArchiveMember(Buffer.from('not a zip'), 'report.json'));
  assert.throws(() => readArchiveMember(Buffer.alloc(16 * 1024 * 1024 + 1), 'report.json'));
});

test('retrieval verifies GitHub archive digest before reading members and rechecks cached provenance', (t) => {
  const directory = mkdtempSync(path.join(os.tmpdir(), 'nvbes-ci-download-test-'));
  t.after(() => rmSync(directory, { recursive: true, force: true }));
  writeFileSync(path.join(directory, 'report.json'), '{}');
  execFileSync('zip', ['-q', 'fixture.zip', 'report.json'], { cwd: directory });
  const bytes = readFileSync(path.join(directory, 'fixture.zip'));
  const f = fixture();
  f.metadata.created_at = new Date(Date.now() - 1000).toISOString();
  f.metadata.expires_at = new Date(Date.now() + 60000).toISOString();
  f.metadata.size_in_bytes = bytes.length;
  f.metadata.digest = `sha256:${createHash('sha256').update(bytes).digest('hex')}`;
  const calls = [];
  const api = (endpoint) => {
    calls.push(endpoint);
    assert(endpoint.startsWith('repos/nvbes-org/nvbes/actions/artifacts/456'));
    return endpoint.endsWith('/zip') ? bytes : Buffer.from(JSON.stringify(f.metadata));
  };
  const read = githubArtifactReader('nvbes-org/nvbes', { api });
  const input = { reference: f.reference, run: f.run, sha: f.sha };
  assert.equal(read(input).toString(), '{}');
  assert.equal(read(input).toString(), '{}');
  assert.equal(calls.length, 2);
  assert.throws(() => read({ ...input, sha: 'c'.repeat(40) }), /another SHA/u);
  assert.throws(() => read({ ...input, run: { ...f.run, id: 124 } }), /another run/u);
  f.metadata.digest = `sha256:${'0'.repeat(64)}`;
  assert.throws(() => githubArtifactReader('nvbes-org/nvbes', { api })(input), /digest mismatch/u);
  f.metadata.size_in_bytes++;
  assert.throws(() => githubArtifactReader('nvbes-org/nvbes', { api })(input), /size mismatch/u);
});
