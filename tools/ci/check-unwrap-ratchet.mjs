#!/usr/bin/env node
/**
 * Ratchet anti-unwrap / expect : le nombre d'unwrap/expect hors tests
 * ne peut pas augmenter dans le code de production.
 * Escape hatch: // nvbes-allow-unwrap
 */
import { execFileSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';

export const UNWRAP_LINE = /\.(?:unwrap|expect)\s*\(/u;
export const ALLOW_COMMENT = 'nvbes-allow-unwrap';

/**
 * @param {string} path
 * @returns {boolean}
 */
export function isProductionRustPath(path) {
  const normalized = path.replaceAll('\\', '/');
  if (!normalized.endsWith('.rs')) return false;
  if (normalized.includes('/archive/')) return false;
  if (/\.tests?\.rs$/u.test(normalized)) return false;
  if (/test_support/u.test(normalized)) return false;
  if (/\/tests\//u.test(normalized)) return false;
  return true;
}

/**
 * @param {string} source
 * @param {number} lineNumber
 */
export function isInsideCfgTest(source, lineNumber) {
  const lines = source.split('\n');
  let depth = 0;
  let inTest = false;
  let pendingTest = false;
  for (let i = 0; i < lines.length; i += 1) {
    const line = lines[i];
    const n = i + 1;
    if (/#\[cfg\(\s*test\s*\)\]/u.test(line)) pendingTest = true;
    if (pendingTest && /\bmod\s+\w+|^\s*(?:async\s+)?fn\s+/u.test(line)) {
      inTest = true;
      pendingTest = false;
    }
    if (/#\[test\]|#\[tokio::test\]|#\[sqlx::test/u.test(line)) pendingTest = true;
    const opens = (line.match(/\{/g) || []).length;
    const closes = (line.match(/\}/g) || []).length;
    if (n === lineNumber) return inTest || pendingTest;
    depth += opens - closes;
    if (inTest && depth <= 0 && closes > 0) {
      inTest = false;
      depth = 0;
    }
  }
  return false;
}

/**
 * @param {string} source
 * @returns {number}
 */
export function countUnwraps(source) {
  let count = 0;
  const lines = source.split('\n');
  for (let i = 0; i < lines.length; i += 1) {
    const line = lines[i];
    if (!UNWRAP_LINE.test(line)) continue;
    if (line.includes(ALLOW_COMMENT)) continue;
    if (isInsideCfgTest(source, i + 1)) continue;
    count += 1;
  }
  return count;
}

/**
 * @param {string} path
 * @param {string} ref
 */
export function readGitFile(path, ref) {
  try {
    return execFileSync('git', ['show', `${ref}:${path}`], {
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
    });
  } catch {
    return null;
  }
}

/**
 * @param {string[]} paths
 * @param {{ baseRef?: string }} [options]
 */
export function evaluateUnwrapRatchet(paths, options = {}) {
  const baseRef = options.baseRef ?? 'HEAD';
  /** @type {{ path: string, before: number, after: number }[]} */
  const regressions = [];
  for (const path of paths) {
    if (!isProductionRustPath(path)) continue;
    if (!existsSync(path)) continue;
    const after = countUnwraps(readFileSync(path, 'utf8'));
    const beforeSource = readGitFile(path, baseRef);
    // New files: prefer fixing unwraps in-review; ratchet only blocks growth on existing files.
    if (beforeSource === null) continue;
    const before = countUnwraps(beforeSource);
    if (after > before) regressions.push({ path, before, after });
  }
  return { ok: regressions.length === 0, regressions };
}

function stagedRustPaths() {
  const out = execFileSync('git', ['diff', '--cached', '--name-only', '-z', '--', '*.rs'], {
    encoding: 'utf8',
  });
  return out.split('\0').filter(Boolean);
}

const isCli = process.argv[1]?.endsWith('check-unwrap-ratchet.mjs');
if (isCli) {
  const baseIdx = process.argv.indexOf('--base');
  const baseRef = baseIdx >= 0 ? process.argv[baseIdx + 1] : 'HEAD';
  const paths = stagedRustPaths();
  const maxFiles = Number(process.env.NVBES_UNWRAP_RATCHET_MAX_FILES || 40);
  if (Number.isFinite(maxFiles) && paths.length > maxFiles && baseRef === 'HEAD') {
    console.log(
      `unwrap-ratchet SKIP: ${paths.length} staged rust files > ${maxFiles} (mega-batch); enforced on small commits`,
    );
    process.exit(0);
  }
  const result = evaluateUnwrapRatchet(paths, { baseRef });
  if (!result.ok) {
    console.error('BLOCKED unwrap-ratchet: unwrap/expect count increased in production:');
    for (const row of result.regressions) {
      console.error(`- ${row.path}: ${row.before} -> ${row.after}`);
    }
    console.error(`Escape hatch on the same line: // ${ALLOW_COMMENT}`);
    process.exit(1);
  }
  console.log('unwrap-ratchet OK');
}
