import assert from 'node:assert/strict';
import { generateKeyPairSync, sign } from 'node:crypto';
import { mkdtempSync, rmSync, writeFileSync, symlinkSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { test } from 'node:test';
import { confinedRead, sha256, verifyBundle } from './v1-bundle-verification.mjs';

function fixture(t) {
  const directory = mkdtempSync(path.join(os.tmpdir(), 'nvbes-v1-evidence-'));
  t.after(() => rmSync(directory, { recursive: true, force: true }));
  const { privateKey, publicKey: key } = generateKeyPairSync('ed25519');
  const publicKey = key.export({ type: 'spki', format: 'pem' });
  writeFileSync(path.join(directory, 'result.json'), '{}');
  const run = {
    repository: { full_name: 'nvbes-org/nvbes' },
    head_sha: 'a'.repeat(40),
    path: '.github/workflows/ci.yml',
    status: 'completed',
    conclusion: 'success',
    run_attempt: 1,
    event: 'push',
  };
  const bundle = {
    sha: run.head_sha,
    artifacts: [{ path: 'result.json', sha256: sha256('{}') }],
    results: [
      {
        artifacts: ['result.json'],
        producer: {
          kind: 'github-actions',
          repository: 'nvbes-org/nvbes',
          workflow: run.path,
          runId: 123,
        },
      },
    ],
  };
  const bytes = Buffer.from(JSON.stringify(bundle));
  return {
    input: {
      bytes,
      signature: sign(null, bytes, privateKey),
      publicKey,
      publicKeyDigest: sha256(publicKey),
      artifactDirectory: directory,
      repository: 'nvbes-org/nvbes',
      workflows: [run.path],
      getRun: () => run,
      units: [{ name: 'fixture-rust', language: 'rust' }],
    },
    directory,
    run,
    bundle,
    privateKey,
  };
}

test('verifies signature, artifact and GitHub run provenance', (t) => {
  const { input } = fixture(t);
  assert.equal(verifyBundle(input).sha, 'a'.repeat(40));
});

for (const [name, mutate] of [
  [
    'packet tampering',
    ({ input }) => {
      input.bytes = Buffer.from('{}');
    },
  ],
  [
    'wrong trusted key',
    ({ input }) => {
      input.publicKeyDigest = '0'.repeat(64);
    },
  ],
  [
    'artifact tampering',
    ({ directory }) => {
      writeFileSync(path.join(directory, 'result.json'), 'changed');
    },
  ],
  [
    'wrong workflow SHA',
    ({ run }) => {
      run.head_sha = 'b'.repeat(40);
    },
  ],
  [
    'failed workflow',
    ({ run }) => {
      run.conclusion = 'failure';
    },
  ],
  [
    'fork event',
    ({ run }) => {
      run.event = 'pull_request';
    },
  ],
  [
    'rerun',
    ({ run }) => {
      run.run_attempt = 2;
    },
  ],
  [
    'unknown workflow',
    ({ input }) => {
      input.workflows = [];
    },
  ],
]) {
  test(`rejects ${String(name)}`, (t) => {
    const f = fixture(t);
    mutate(f);
    assert.throws(() => verifyBundle(f.input));
  });
}

test('rejects traversal and escaping symlinks', (t) => {
  const { directory } = fixture(t);
  assert.throws(() => confinedRead(directory, '../result.json'), /traversal/u);
  symlinkSync(os.tmpdir(), path.join(directory, 'outside'));
  assert.throws(() => confinedRead(directory, 'outside'), /escapes/u);
});

test('signed packets cannot bypass raw TypeScript measurement validation', (t) => {
  const f = fixture(t);
  const source = 'export const value = true;';
  const file = 'libs/ts/example/src/index.ts';
  f.input.units = [
    {
      name: '@nvbes/example',
      language: 'typescript',
      sourceFiles: {
        [file]: { sha256: sha256(source), runtime: true },
      },
    },
  ];
  const coverage = {
    [file]: {
      lines: { total: 10000, covered: 8999, skipped: 0 },
      branches: { total: 1, covered: 1, skipped: 0 },
    },
  };
  const mutation = {
    schemaVersion: '1.0',
    files: {
      [file]: {
        source,
        mutants: [{ id: '0', status: 'Killed' }],
      },
    },
  };
  for (const [name, report] of [
    ['coverage.json', coverage],
    ['mutation.json', mutation],
  ]) {
    const bytes = JSON.stringify(report);
    writeFileSync(path.join(f.directory, name), bytes);
    f.bundle.artifacts.push({ path: name, sha256: sha256(bytes) });
  }
  const measurement = {
    unit: '@nvbes/example',
    lines: 90,
    branches: 100,
    mutation: 100,
    artifacts: ['coverage.json', 'mutation.json'],
    reports: { 'istanbul-summary': 'coverage.json', stryker: 'mutation.json' },
  };
  f.bundle.measurements = [measurement];
  const check = () => {
    f.input.bytes = Buffer.from(JSON.stringify(f.bundle));
    f.input.signature = sign(null, f.input.bytes, f.privateKey);
    return verifyBundle(f.input);
  };
  assert.throws(check, /declared lines differs/u);
  measurement.lines = 89.99;
  assert.equal(check().measurements[0].lines, 89.99);
  delete measurement.reports;
  assert.throws(check, /verified istanbul-summary/u);
  delete f.input.units;
  assert.throws(check, /scope required/u);
});
