#!/usr/bin/env node
/**
 * Refuse les assertions faibles ajoutées dans le code de production :
 *   assert!(x.is_err()) / assert!(x.is_ok())
 * Les fichiers *.tests.rs restent hors scope (campagne de couverture).
 * Escape hatch: // nvbes-allow-weak-assert
 */
import { execFileSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { isInsideCfgTest, isProductionRustPath } from './check-unwrap-ratchet.mjs';

export const WEAK_ASSERT_PATTERN = /^\+.*\bassert!\(\s*(?:.*\.)?(?:is_err|is_ok)\(\s*\)\s*\)/u;

export const ALLOW_COMMENT = 'nvbes-allow-weak-assert';

/**
 * @param {string} diffText
 * @param {(path: string) => string | null} [readFile]
 * @returns {{ path: string, line: string }[]}
 */
export function findWeakAssertionsInDiff(diffText, readFile = defaultRead) {
  /** @type {{ path: string, line: string }[]} */
  const hits = [];
  let currentPath = '';
  let newLine = 0;
  for (const raw of diffText.split('\n')) {
    if (raw.startsWith('+++ b/')) {
      currentPath = raw.slice('+++ b/'.length).trim();
      newLine = 0;
      continue;
    }
    const hunk = /^@@ -\d+(?:,\d+)? \+(\d+)(?:,\d+)? @@/u.exec(raw);
    if (hunk) {
      newLine = Number(hunk[1]) - 1;
      continue;
    }
    if (raw.startsWith('\\') || raw.startsWith('diff ')) continue;
    if (raw.startsWith('+') && !raw.startsWith('+++')) {
      newLine += 1;
      if (!isProductionRustPath(currentPath)) continue;
      if (raw.includes(ALLOW_COMMENT)) continue;
      if (!WEAK_ASSERT_PATTERN.test(raw)) continue;
      const source = readFile(currentPath);
      if (source && isInsideCfgTest(source, newLine)) continue;
      hits.push({ path: currentPath, line: raw.slice(1).trim() });
      continue;
    }
    if (raw.startsWith('-') && !raw.startsWith('---')) continue;
    if (raw.startsWith(' ') || raw === '') newLine += 1;
  }
  return hits;
}

/**
 * @param {string} path
 * @returns {string | null}
 */
function defaultRead(path) {
  if (!existsSync(path)) return null;
  return readFileSync(path, 'utf8');
}

/**
 * @param {{ baseRef?: string, headRef?: string, staged?: boolean }} [options]
 * @returns {{ ok: boolean, hits: { path: string, line: string }[] }}
 */
export function checkWeakAssertions(options = {}) {
  let diffText = '';
  if (options.staged) {
    diffText = execFileSync('git', ['diff', '--cached', '--', '*.rs', ':(exclude)archive/**'], {
      encoding: 'utf8',
      maxBuffer: 64 * 1024 * 1024,
    });
  } else {
    const baseRef = options.baseRef ?? 'origin/main';
    const headRef = options.headRef ?? 'HEAD';
    try {
      diffText = execFileSync(
        'git',
        ['diff', `${baseRef}...${headRef}`, '--', '*.rs', ':(exclude)archive/**'],
        { encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 },
      );
    } catch {
      diffText = execFileSync('git', ['diff', '--cached', '--', '*.rs'], {
        encoding: 'utf8',
        maxBuffer: 64 * 1024 * 1024,
      });
    }
  }
  const hits = findWeakAssertionsInDiff(diffText);
  return { ok: hits.length === 0, hits };
}

const isCli = process.argv[1]?.endsWith('check-weak-assertions.mjs');
if (isCli) {
  const baseIdx = process.argv.indexOf('--base');
  const headIdx = process.argv.indexOf('--head');
  const staged = process.argv.includes('--staged');
  const result = checkWeakAssertions({
    baseRef: baseIdx >= 0 ? process.argv[baseIdx + 1] : 'origin/main',
    headRef: headIdx >= 0 ? process.argv[headIdx + 1] : 'HEAD',
    staged,
  });
  if (!result.ok) {
    console.error('BLOCKED weak-assertions: prefer matches! / assert_eq! on error variants:');
    for (const hit of result.hits) console.error(`- ${hit.path}: ${hit.line}`);
    console.error(`Escape hatch on the same line: // ${ALLOW_COMMENT}`);
    process.exit(1);
  }
  console.log('weak-assertions OK');
}
