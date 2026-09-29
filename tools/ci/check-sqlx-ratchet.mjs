#!/usr/bin/env node
/**
 * Ratchet SQLx : le nombre de sqlx::query / query_as / query_scalar runtime
 * (hors query!) ne peut pas augmenter dans le code de production.
 * Escape hatch ligne : // nvbes-allow-runtime-sql
 */
import { execFileSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { isInsideCfgTest, isProductionRustPath } from './check-unwrap-ratchet.mjs';

export const RUNTIME_SQL_LINE = /\bsqlx::(?:query|query_as|query_scalar)\s*(?:::<[^>]*>)?\s*\(/u;
export const ALLOW_COMMENT = 'nvbes-allow-runtime-sql';

/**
 * @param {string} source
 * @returns {number}
 */
export function countRuntimeSql(source) {
  let count = 0;
  const lines = source.split('\n');
  for (let i = 0; i < lines.length; i += 1) {
    const line = lines[i];
    if (!RUNTIME_SQL_LINE.test(line)) continue;
    if (line.includes(ALLOW_COMMENT)) continue;
    if (isInsideCfgTest(source, i + 1)) continue;
    count += 1;
  }
  return count;
}

/**
 * @param {string} path
 * @param {string} ref
 * @returns {string | null}
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
export function evaluateSqlxRatchet(paths, options = {}) {
  const baseRef = options.baseRef ?? 'HEAD';
  /** @type {{ path: string, before: number, after: number }[]} */
  const regressions = [];
  for (const path of paths) {
    if (!isProductionRustPath(path)) continue;
    if (!existsSync(path)) continue;
    const after = countRuntimeSql(readFileSync(path, 'utf8'));
    const beforeSource = readGitFile(path, baseRef);
    // New production files are admitted by the dedicated query! migration campaign.
    if (beforeSource === null) continue;
    const before = countRuntimeSql(beforeSource);
    if (after > before) regressions.push({ path, before, after });
  }
  return { ok: regressions.length === 0, regressions };
}

/**
 * @returns {string[]}
 */
function stagedRustPaths() {
  const out = execFileSync('git', ['diff', '--cached', '--name-only', '-z', '--', '*.rs'], {
    encoding: 'utf8',
  });
  return out.split('\0').filter(Boolean);
}

const isCli = process.argv[1]?.endsWith('check-sqlx-ratchet.mjs');
if (isCli) {
  const baseIdx = process.argv.indexOf('--base');
  const baseRef = baseIdx >= 0 ? process.argv[baseIdx + 1] : 'HEAD';
  const paths = stagedRustPaths();
  const maxFiles = Number(process.env.NVBES_SQLX_RATCHET_MAX_FILES || 40);
  if (Number.isFinite(maxFiles) && paths.length > maxFiles && baseRef === 'HEAD') {
    console.log(
      `sqlx-ratchet SKIP: ${paths.length} staged rust files > ${maxFiles} (mega-batch); enforced on small commits / --base origin/main`,
    );
    process.exit(0);
  }
  const result = evaluateSqlxRatchet(paths, { baseRef });
  if (!result.ok) {
    console.error('BLOCKED sqlx-ratchet: runtime sqlx::query* count increased:');
    for (const row of result.regressions) {
      console.error(`- ${row.path}: ${row.before} -> ${row.after}`);
    }
    console.error('Prefer sqlx::query! / query_as!. Escape hatch: // nvbes-allow-runtime-sql');
    process.exit(1);
  }
  console.log('sqlx-ratchet OK');
}
