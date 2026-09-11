import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { lstatSync, mkdirSync, readFileSync, renameSync, writeFileSync } from 'node:fs';
import path from 'node:path';

const digest = (bytes) => createHash('sha256').update(bytes).digest('hex');

export function mutationRunPaths(name, directory) {
  assert(/^[a-z][a-z0-9-]*$/u.test(name ?? ''), 'Invalid mutation package');
  if (directory === undefined) {
    return { report: `.temp/typescript/${name}/mutation.json`, sandbox: `.temp/stryker-${name}` };
  }
  assert(
    new RegExp(`^\\.temp/mutation-runs/${name}-[A-Za-z0-9]{6}$`, 'u').test(directory),
    'Invalid mutation run directory',
  );
  return { report: `${directory}/mutation.json`, sandbox: `${directory}/sandbox` };
}

export function mutationCheckout(root) {
  const git = (args) => execFileSync('git', args, { cwd: root, maxBuffer: 64 * 1024 * 1024 });
  const diff = git(['diff', '--no-ext-diff', '--no-textconv', '--binary', 'HEAD']);
  // Untracked tests and configuration can change execution without changing HEAD.
  // Persist only their digest, never file contents or environment values.
  const untracked = git([
    'ls-files',
    '--others',
    '--exclude-standard',
    '-z',
    '--',
    'libs/ts',
    'tools/test-summary',
    'docs/testing',
  ])
    .toString()
    .split('\0')
    .filter(Boolean)
    .sort();
  const hash = createHash('sha256').update(diff);
  for (const file of untracked) {
    hash
      .update(file)
      .update('\0')
      .update(readFileSync(path.join(root, file)))
      .update('\0');
  }
  return {
    sha: git(['rev-parse', 'HEAD']).toString().trim(),
    trackedChanges: git(['status', '--porcelain', '--untracked-files=no']).toString().trim() !== '',
    checkoutSha256: hash.digest('hex'),
  };
}

export function publishMutationReport({ root, name, report, before, after, validate }) {
  assert.equal(report, mutationRunPaths(name, path.posix.dirname(report)).report);
  assert.deepEqual(after, before, 'Mutation checkout changed during execution');
  const source = path.join(root, report);
  assert(lstatSync(source).isFile(), 'Mutation report must be a regular file');
  const bytes = readFileSync(source);
  // Validate precisely the bytes being archived and published, not an earlier read.
  validate(JSON.parse(bytes.toString()));
  const destination = path.join(root, '.temp/typescript', name, 'mutation.json');
  mkdirSync(path.dirname(destination), { recursive: true });
  // Unique temporary files avoid concurrent publication mixing two reports.
  const pending = `${destination}.${path.basename(path.dirname(source))}.pending`;
  writeFileSync(pending, bytes, { flag: 'wx', mode: 0o600 });
  renameSync(pending, destination);
  return { report, sha256: digest(bytes), bytes: bytes.length };
}

export function verifyMutationArtifact(root, artifact) {
  assert(
    /^\.temp\/mutation-runs\/[a-z][a-z0-9-]*-[A-Za-z0-9]{6}\/mutation\.json$/u.test(
      artifact.report,
    ),
    'Invalid mutation artifact path',
  );
  assert(
    lstatSync(path.join(root, artifact.report)).isFile(),
    'Mutation artifact must be a regular file',
  );
  const bytes = readFileSync(path.join(root, artifact.report));
  assert.equal(bytes.length, artifact.bytes, 'Mutation artifact size changed');
  assert.equal(digest(bytes), artifact.sha256, 'Mutation artifact digest changed');
  return JSON.parse(bytes.toString());
}
