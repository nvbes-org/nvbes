#!/usr/bin/env node
/**
 * Mutation gate sur le diff (cargo-mutants --in-diff).
 * Zero MissedMutant / Timeout sur le code modifié ; skip si aucun .rs touché.
 */
import { execFileSync, spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

export const CRITICAL_PACKAGES = new Set([
  'nvbes-core',
  'nvbes-billing',
  'nvbes-audit',
  'nvbes-trust-risk',
  'nvbes-identity-service',
  'nvbes-account-service',
  'nvbes-billing-service',
  'nvbes-billing-worker',
  'nvbes-trust-risk-service',
]);

/**
 * @param {string} baseRef
 * @param {string} headRef
 * @returns {string[]}
 */
export function rustFilesInDiff(baseRef, headRef) {
  const out = execFileSync(
    'git',
    ['diff', '--name-only', `${baseRef}...${headRef}`, '--', '*.rs', ':(exclude)archive/**'],
    { encoding: 'utf8' },
  );
  return out
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean);
}

/**
 * @param {unknown} outcomes
 * @returns {{ missed: number, timeout: number, caught: number, tested: number, criticalMissed: string[] }}
 */
export function summarizeDiffOutcomes(outcomes) {
  const list = Array.isArray(outcomes)
    ? outcomes
    : Array.isArray(/** @type {{ outcomes?: unknown }} */ (outcomes)?.outcomes)
      ? /** @type {{ outcomes: unknown[] }} */ (outcomes).outcomes
      : [];
  let missed = 0;
  let timeout = 0;
  let caught = 0;
  /** @type {string[]} */
  const criticalMissed = [];
  for (const outcome of list) {
    const record = /** @type {Record<string, unknown>} */ (outcome);
    const summary = record.summary;
    const scenario = /** @type {Record<string, unknown> | undefined} */ (record.scenario);
    const mutant = scenario && /** @type {Record<string, unknown>} */ (scenario.Mutant);
    const pkg = typeof mutant?.package === 'string' ? mutant.package : '';
    const label = (value) =>
      typeof value === 'string' || typeof value === 'number' ? String(value) : 'mutant';
    if (summary === 'CaughtMutant') caught += 1;
    else if (summary === 'MissedMutant') {
      missed += 1;
      if (CRITICAL_PACKAGES.has(pkg)) {
        criticalMissed.push(`${pkg}: ${label(mutant?.function ?? mutant?.file)}`);
      }
    } else if (summary === 'Timeout') {
      timeout += 1;
      if (CRITICAL_PACKAGES.has(pkg)) {
        criticalMissed.push(`${pkg}: timeout ${label(mutant?.function)}`);
      }
    }
  }
  return { missed, timeout, caught, tested: caught + missed + timeout, criticalMissed };
}

/**
 * @param {{
 *   baseRef?: string,
 *   headRef?: string,
 *   outRoot?: string,
 *   timeoutSec?: number,
 *   dryRun?: boolean,
 * }} [options]
 */
export function runMutationInDiff(options = {}) {
  const baseRef = options.baseRef ?? 'origin/main';
  const headRef = options.headRef ?? 'HEAD';
  const outRoot = resolve(options.outRoot ?? '.temp/rust-in-diff');
  const timeoutSec = options.timeoutSec ?? 300;
  const files = rustFilesInDiff(baseRef, headRef);
  const maxFiles = Number(process.env.NVBES_MUTATION_DIFF_MAX_FILES || 40);
  if (files.length === 0) {
    return { skipped: true, reason: 'no rust files in diff', files, ok: true };
  }
  if (Number.isFinite(maxFiles) && files.length > maxFiles) {
    return {
      skipped: true,
      reason: `${files.length} rust files in diff exceeds NVBES_MUTATION_DIFF_MAX_FILES=${maxFiles}; split the change or raise the cap`,
      files,
      ok: true,
    };
  }

  mkdirSync(outRoot, { recursive: true });
  const diffPath = resolve(outRoot, 'range.diff');
  const diff = execFileSync('git', ['diff', `${baseRef}...${headRef}`, '--', '*.rs'], {
    encoding: 'utf8',
    maxBuffer: 64 * 1024 * 1024,
  });
  writeFileSync(diffPath, diff);

  if (options.dryRun) {
    return { skipped: false, dryRun: true, files, diffPath, ok: true };
  }

  rmSync(resolve(outRoot, 'mutants.out'), { recursive: true, force: true });

  const env = { ...process.env };
  delete env.CARGO_TARGET_DIR;

  const run = spawnSync(
    'cargo',
    [
      'mutants',
      '--locked',
      '--in-diff',
      diffPath,
      '--timeout',
      String(timeoutSec),
      '--no-times',
      '-o',
      outRoot,
    ],
    { stdio: 'inherit', env },
  );

  const outcomesPath = resolve(outRoot, 'mutants.out', 'outcomes.json');
  if (!existsSync(outcomesPath)) {
    return {
      skipped: false,
      ok: false,
      files,
      error: `missing outcomes at ${outcomesPath} (exit ${run.status})`,
    };
  }
  const outcomes = JSON.parse(readFileSync(outcomesPath, 'utf8'));
  const summary = summarizeDiffOutcomes(outcomes);
  const ok = summary.missed === 0 && summary.timeout === 0;
  return { skipped: false, ok, files, summary, outcomesPath };
}

const isCli = process.argv[1]?.endsWith('mutation-in-diff.mjs');
if (isCli) {
  const baseIdx = process.argv.indexOf('--base');
  const headIdx = process.argv.indexOf('--head');
  const dry = process.argv.includes('--dry-run');
  const result = runMutationInDiff({
    baseRef: baseIdx >= 0 ? process.argv[baseIdx + 1] : 'origin/main',
    headRef: headIdx >= 0 ? process.argv[headIdx + 1] : 'HEAD',
    dryRun: dry,
  });
  if (result.skipped) {
    console.log(`mutation-in-diff SKIP: ${result.reason}`);
    process.exit(0);
  }
  if (result.dryRun) {
    console.log(`mutation-in-diff dry-run: ${result.files.length} rust file(s)`);
    process.exit(0);
  }
  if (result.error) {
    console.error(`BLOCKED mutation-in-diff: ${result.error}`);
    process.exit(1);
  }
  console.log(
    `mutation-in-diff: tested=${result.summary.tested} caught=${result.summary.caught} missed=${result.summary.missed} timeout=${result.summary.timeout}`,
  );
  if (!result.ok) {
    console.error('BLOCKED mutation-in-diff: surviving mutants on changed code');
    for (const entry of result.summary.criticalMissed) console.error(`- ${entry}`);
    process.exit(1);
  }
  console.log('mutation-in-diff OK');
}
