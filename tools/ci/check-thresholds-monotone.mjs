#!/usr/bin/env node
/**
 * Cliquet anti-Goodhart : un seuil de couverture / mutation / condition
 * ne peut jamais baisser par rapport à origin/main, et un crate ne peut
 * pas passer de "gated" à "excluded" sans intervention humaine.
 */
import { execFileSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';

export const THRESHOLD_FILES = [
  'docs/testing/rust-coverage-thresholds.json',
  'docs/testing/rust-mutation-thresholds.json',
  'docs/testing/rust-condition-thresholds.json',
];

/**
 * @param {unknown} value
 * @returns {value is Record<string, unknown>}
 */
function isRecord(value) {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

/**
 * @param {Record<string, unknown>} crates
 * @param {string} name
 * @returns {number | null}
 */
export function crateThreshold(crates, name) {
  const entry = crates[name];
  if (!isRecord(entry)) return null;
  if (typeof entry.threshold === 'number') return entry.threshold;
  if (isRecord(entry.threshold) && typeof entry.threshold.lines === 'number') {
    return entry.threshold.lines;
  }
  return null;
}

/**
 * @param {unknown} config
 * @returns {{ crates: Record<string, unknown>, excluded: Set<string> }}
 */
export function normalizeThresholdConfig(config) {
  if (!isRecord(config)) return { crates: {}, excluded: new Set() };
  const crates = isRecord(config.crates) ? config.crates : {};
  const excluded = new Set();
  if (Array.isArray(config.excluded)) {
    for (const entry of config.excluded) {
      if (isRecord(entry) && typeof entry.crate === 'string') excluded.add(entry.crate);
    }
  }
  return { crates, excluded };
}

/**
 * @param {unknown} baseConfig
 * @param {unknown} headConfig
 * @param {string} label
 * @returns {string[]}
 */
export function evaluateThresholdMonotonicity(baseConfig, headConfig, label) {
  const failures = [];
  const base = normalizeThresholdConfig(baseConfig);
  const head = normalizeThresholdConfig(headConfig);

  for (const name of Object.keys(base.crates)) {
    if (head.excluded.has(name) && !base.excluded.has(name)) {
      failures.push(
        `${label}: crate ${name} moved from gated to excluded (anti-Goodhart: forbidden)`,
      );
      continue;
    }
    if (!(name in head.crates) && !head.excluded.has(name)) {
      failures.push(`${label}: crate ${name} removed from gate without exclusion reason`);
      continue;
    }
    const baseValue = crateThreshold(base.crates, name);
    const headValue = crateThreshold(head.crates, name);
    if (baseValue !== null && headValue !== null && headValue < baseValue) {
      failures.push(
        `${label}: ${name} threshold lowered ${baseValue} -> ${headValue} (cliqets are monotone)`,
      );
    }
  }
  return failures;
}

/**
 * @param {string} path
 * @param {string} baseRef
 * @returns {unknown | null}
 */
export function readBaseJson(path, baseRef) {
  try {
    const text = execFileSync('git', ['show', `${baseRef}:${path}`], {
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    return JSON.parse(text);
  } catch {
    return null;
  }
}

/**
 * @param {string} path
 * @returns {unknown}
 */
export function readHeadJson(path) {
  return JSON.parse(readFileSync(path, 'utf8'));
}

/**
 * @param {{ files?: string[], baseRef?: string, cwd?: string }} [options]
 * @returns {{ ok: boolean, failures: string[], checked: string[] }}
 */
export function checkThresholdsMonotone(options = {}) {
  const files = options.files ?? THRESHOLD_FILES;
  const baseRef = options.baseRef ?? 'origin/main';
  const cwd = options.cwd ?? process.cwd();
  const failures = [];
  const checked = [];

  for (const relative of files) {
    const absolute = resolve(cwd, relative);
    if (!existsSync(absolute)) {
      failures.push(`${relative}: missing on HEAD`);
      continue;
    }
    const head = readHeadJson(absolute);
    const base = readBaseJson(relative, baseRef);
    if (base === null) {
      checked.push(`${relative} (no base at ${baseRef}; skip)`);
      continue;
    }
    checked.push(relative);
    failures.push(...evaluateThresholdMonotonicity(base, head, relative));
  }

  return { ok: failures.length === 0, failures, checked };
}

const isCli = process.argv[1]?.endsWith('check-thresholds-monotone.mjs');
if (isCli) {
  const baseIdx = process.argv.indexOf('--base');
  const baseRef = baseIdx >= 0 ? process.argv[baseIdx + 1] : 'origin/main';
  const result = checkThresholdsMonotone({ baseRef });
  for (const entry of result.checked) console.log(`checked: ${entry}`);
  if (!result.ok) {
    console.error('BLOCKED thresholds-monotone:');
    for (const failure of result.failures) console.error(`- ${failure}`);
    process.exit(1);
  }
  console.log('thresholds-monotone OK');
}
