#!/usr/bin/env node
/**
 * Lint every non-merge commit in a range with the same rules as CI commitlint.
 * Catches body-max-line-length and other rules even when commit-msg was skipped
 * via --no-verify.
 */
import { spawnSync } from 'node:child_process';

function arg(name, fallback) {
  const index = process.argv.indexOf(name);
  if (index >= 0 && process.argv[index + 1]) return process.argv[index + 1];
  return fallback;
}

const base = arg('--base', 'origin/main');
const head = arg('--head', 'HEAD');
const range = `${base}..${head}`;

const listed = spawnSync('git', ['rev-list', '--no-merges', range], {
  encoding: 'utf8',
});
if (listed.status !== 0) {
  process.stderr.write(listed.stderr || `git rev-list failed for ${range}\n`);
  process.exit(listed.status ?? 1);
}

const commits = listed.stdout
  .split('\n')
  .map((line) => line.trim())
  .filter(Boolean);

if (commits.length === 0) {
  console.log(`commitlint range: ok (0 commits in ${range})`);
  process.exit(0);
}

let failed = 0;
for (const commit of commits) {
  const message = spawnSync('git', ['show', '-s', '--format=%B', commit], {
    encoding: 'utf8',
  });
  if (message.status !== 0) {
    process.stderr.write(message.stderr || `git show failed for ${commit}\n`);
    failed += 1;
    continue;
  }
  const lint = spawnSync('pnpm', ['exec', 'commitlint', '--strict', '--verbose'], {
    input: message.stdout,
    encoding: 'utf8',
    shell: process.platform === 'win32',
  });
  if (lint.stdout) process.stdout.write(lint.stdout);
  if (lint.stderr) process.stderr.write(lint.stderr);
  if (lint.status !== 0) {
    process.stderr.write(`commitlint failed for ${commit}\n`);
    failed += 1;
  }
}

if (failed > 0) {
  process.stderr.write(
    `commitlint range: ${failed}/${commits.length} commit(s) failed in ${range}\n`,
  );
  process.exit(1);
}

console.log(`commitlint range: ok (${commits.length} commits in ${range})`);
