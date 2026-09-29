#!/usr/bin/env node
/**
 * Refuse unsigned commits by default.
 *
 * Accepts SSH or OpenPGP signatures embedded in the commit object (same
 * surface as GitHub `required_signatures`). Modes:
 *   --config              require local git signing setup (pre-commit)
 *   --base X --head Y     require every non-merge commit in X..Y is signed
 */
import { spawnSync } from 'node:child_process';

const SSH_MARKER = '-----BEGIN SSH SIGNATURE-----';
const PGP_MARKER = '-----BEGIN PGP SIGNATURE-----';

export function arg(name, fallback, argv = process.argv) {
  const index = argv.indexOf(name);
  if (index >= 0 && argv[index + 1]) return argv[index + 1];
  return fallback;
}

export function gitConfig(key, cwd = process.cwd()) {
  const result = spawnSync('git', ['config', '--get', key], {
    encoding: 'utf8',
    cwd,
  });
  if (result.status !== 0) return null;
  return result.stdout.trim() || null;
}

export function evaluateSigningConfig({ gpgsign, gpgFormat, signingKey } = {}) {
  const reasons = [];
  const signEnabled =
    gpgsign === 'true' || gpgsign === '1' || gpgsign === 'yes' || gpgsign === 'on';
  if (!signEnabled) {
    reasons.push('commit.gpgsign must be true (unsigned commits are refused)');
  }
  if (!signingKey) {
    reasons.push('user.signingkey must be set (SSH public key path or OpenPGP key id)');
  }
  const format = gpgFormat ?? 'openpgp';
  if (format !== 'ssh' && format !== 'openpgp' && format !== 'x509') {
    reasons.push(`gpg.format must be ssh or openpgp (got ${format})`);
  }
  return { ok: reasons.length === 0, reasons, format };
}

export function classifyCommitSignature(rawCommit) {
  if (typeof rawCommit !== 'string' || rawCommit.length === 0) return 'missing';
  if (rawCommit.includes(SSH_MARKER)) return 'ssh';
  if (rawCommit.includes(PGP_MARKER)) return 'pgp';
  if (rawCommit.includes('gpgsig ')) return 'unknown';
  return 'none';
}

export function evaluateSignedCommits(classifications) {
  const failures = [];
  for (const { sha, kind } of classifications) {
    if (kind === 'ssh' || kind === 'pgp') continue;
    if (kind === 'none' || kind === 'missing') {
      failures.push({ sha, kind, message: 'unsigned commit' });
    } else {
      failures.push({ sha, kind, message: 'unrecognized commit signature' });
    }
  }
  return { ok: failures.length === 0, failures };
}

function readCommitObject(sha) {
  const result = spawnSync('git', ['cat-file', '-p', sha], { encoding: 'utf8' });
  if (result.status !== 0) {
    throw new Error(result.stderr || `git cat-file failed for ${sha}`);
  }
  return result.stdout;
}

function listCommits(base, head) {
  const range = `${base}..${head}`;
  const listed = spawnSync('git', ['rev-list', '--no-merges', range], {
    encoding: 'utf8',
  });
  if (listed.status !== 0) {
    process.stderr.write(listed.stderr || `git rev-list failed for ${range}\n`);
    process.exit(listed.status ?? 1);
  }
  return listed.stdout
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean);
}

function setupHint() {
  return [
    'Configure commit signing (SSH recommended, OpenPGP accepted):',
    '  git config commit.gpgsign true',
    '  # SSH:',
    '  git config gpg.format ssh',
    '  git config user.signingkey ~/.ssh/id_ed25519.pub',
    '  # or OpenPGP: git config user.signingkey <KEYID>',
    'Add the same key to GitHub as a Signing key so signatures show as Verified.',
  ].join('\n');
}

function runConfigCheck() {
  const result = evaluateSigningConfig({
    gpgsign: gitConfig('commit.gpgsign'),
    gpgFormat: gitConfig('gpg.format'),
    signingKey: gitConfig('user.signingkey'),
  });
  if (!result.ok) {
    process.stderr.write(`error: commit signing is required\n`);
    for (const reason of result.reasons) {
      process.stderr.write(`  - ${reason}\n`);
    }
    process.stderr.write(`${setupHint()}\n`);
    process.exit(1);
  }
  console.log(`commit signing config: ok (format=${result.format})`);
}

function runRangeCheck(base, head) {
  const commits = listCommits(base, head);
  if (commits.length === 0) {
    console.log(`commit signatures: ok (0 commits in ${base}..${head})`);
    return;
  }
  const classifications = commits.map((sha) => ({
    sha,
    kind: classifyCommitSignature(readCommitObject(sha)),
  }));
  const result = evaluateSignedCommits(classifications);
  if (!result.ok) {
    process.stderr.write(
      `error: ${result.failures.length}/${commits.length} unsigned commit(s) in ${base}..${head}\n`,
    );
    for (const failure of result.failures) {
      process.stderr.write(`  - ${failure.sha.slice(0, 12)}: ${failure.message}\n`);
    }
    process.stderr.write(`${setupHint()}\n`);
    process.exit(1);
  }
  console.log(`commit signatures: ok (${commits.length} commits in ${base}..${head})`);
}

const isCli = process.argv[1]?.endsWith('check-commit-ssh-signing.mjs');
if (isCli) {
  const wantsConfig = process.argv.includes('--config');
  const base = arg('--base', null);
  const head = arg('--head', null);
  if (wantsConfig || (!base && !head)) {
    runConfigCheck();
  }
  if (base || head) {
    runRangeCheck(base ?? 'origin/main', head ?? 'HEAD');
  }
}
