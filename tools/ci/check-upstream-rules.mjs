#!/usr/bin/env node
/**
 * Vérificateur en amont des règles strictes SonarQube, CodeQL et CI.
 * Détecte les anti-patterns bloquants avant commit / invocation SonarCloud :
 *  - SQL : UPDATE / DELETE sans clause WHERE (plsql:DeleteOrUpdateWithoutWhereCheck)
 *  - JS/TS : .sort() sans comparateur explicite (typescript:S2871)
 *  - JS/TS : Math.random() en contexte de production (typescript:S2245)
 *  - JS/TS : parseInt global au lieu de Number.parseInt (typescript:S7773)
 *  - JS/TS : Regex vulnérable ReDoS / backtracking (typescript:S8786)
 *
 * Escape hatch pour cas exceptionnels justifiés : // nvbes-allow-upstream-rule:<rule>
 */

import { execFileSync } from 'node:child_process';

export const ALLOW_PREFIX = 'nvbes-allow-upstream-rule';

/**
 * @typedef {{ file: string, line: number, rule: string, message: string, snippet: string }} Violation
 */

/**
 * @param {string} diffText
 * @returns {Violation[]}
 */
export function findUpstreamViolationsInDiff(diffText) {
  /** @type {Violation[]} */
  const violations = [];
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
      const lineText = raw.slice(1);

      if (lineText.includes(ALLOW_PREFIX)) continue;

      // 1. SQL Checks (migrations & SQL files)
      if (currentPath.endsWith('.sql')) {
        const trimmed = lineText.trim();
        // Check for UPDATE or DELETE statements without WHERE
        const isUpdateWithoutWhere =
          /^\s*UPDATE\s+[a-zA-Z0-9_.]+\s+SET\b/i.test(trimmed) &&
          !/\bWHERE\b/i.test(trimmed) &&
          trimmed.endsWith(';');
        const isDeleteWithoutWhere = /^\s*DELETE\s+FROM\s+[a-zA-Z0-9_.]+\s*;/i.test(trimmed);

        if (isUpdateWithoutWhere || isDeleteWithoutWhere) {
          violations.push({
            file: currentPath,
            line: newLine,
            rule: 'plsql:DeleteOrUpdateWithoutWhereCheck',
            message: 'SQL UPDATE/DELETE statement without WHERE clause.',
            snippet: trimmed,
          });
        }
      }

      // 2. TypeScript / JavaScript Checks (apps/ and libs/ production code)
      const isTsOrJs = /\.(?:ts|tsx|js|mjs|cjs)$/u.test(currentPath);
      const isTooling = currentPath.startsWith('tools/') || currentPath.startsWith('scripts/');
      const isTestFile =
        /\.(?:test|spec)\.(?:ts|tsx|js|mjs)$/u.test(currentPath) ||
        currentPath.includes('/tests/') ||
        currentPath.includes('/__tests__/') ||
        currentPath.includes('/mocks/') ||
        currentPath.includes('/fixtures/');

      const trimmed = lineText.trim();
      const isComment =
        trimmed.startsWith('//') || trimmed.startsWith('*') || trimmed.startsWith('/*');

      if (isTsOrJs && !isTestFile && !isTooling && !isComment) {
        // typescript:S2871 - sort() without comparator
        if (/\.sort\(\s*\)/u.test(lineText)) {
          violations.push({
            file: currentPath,
            line: newLine,
            rule: 'typescript:S2871',
            message:
              'Array .sort() called without comparator. Use .sort((a, b) => a.localeCompare(b)) or (a, b) => a - b.',
            snippet: lineText.trim(),
          });
        }

        // typescript:S2245 - Math.random() in production code
        if (/\bMath\.random\(\)/u.test(lineText)) {
          violations.push({
            file: currentPath,
            line: newLine,
            rule: 'typescript:S2245',
            message:
              'Math.random() in production code. Use crypto.getRandomValues() for CSPRNG/security.',
            snippet: lineText.trim(),
          });
        }

        // typescript:S7773 - global parseInt()
        if (/(?<![A-Za-z0-9_$.])parseInt\s*\(/u.test(lineText)) {
          violations.push({
            file: currentPath,
            line: newLine,
            rule: 'typescript:S7773',
            message: 'Global parseInt() used. Prefer Number.parseInt().',
            snippet: lineText.trim(),
          });
        }

        // typescript:S8786 - Backtracking regex (ReDoS) like /=+$/
        if (/=\+\$/u.test(lineText)) {
          violations.push({
            file: currentPath,
            line: newLine,
            rule: 'typescript:S8786',
            message:
              'Potentially catastrophic backtracking regex (/=+\\$/). Use bounded quantifiers (e.g. /={1,2}\\$/) or string methods.',
            snippet: lineText.trim(),
          });
        }
      }
    }
  }

  return violations;
}

function runCli() {
  const args = process.argv.slice(2);
  let diff = '';
  if (args.includes('--staged')) {
    diff = execFileSync('git', ['diff', '--cached', '--no-color', '--unified=0'], {
      encoding: 'utf8',
    });
  } else if (args.includes('--range')) {
    const idx = args.indexOf('--range');
    const range = args[idx + 1] || 'origin/main...HEAD';
    diff = execFileSync('git', ['diff', '--no-color', '--unified=0', range], { encoding: 'utf8' });
  } else {
    diff = execFileSync('git', ['diff', '--cached', '--no-color', '--unified=0'], {
      encoding: 'utf8',
    });
    if (!diff.trim()) {
      diff = execFileSync('git', ['diff', '--no-color', '--unified=0', 'HEAD~1...HEAD'], {
        encoding: 'utf8',
      });
    }
  }

  const violations = findUpstreamViolationsInDiff(diff);
  if (violations.length > 0) {
    console.error('❌ Upstream SonarQube/CodeQL rule violations detected:');
    for (const v of violations) {
      console.error(`  - [${v.rule}] ${v.file}:${v.line}`);
      console.error(`    ${v.message}`);
      console.error(`    > ${v.snippet}`);
    }
    process.exit(1);
  }
  console.log('upstream-rules OK');
}

if (process.argv[1]?.endsWith('check-upstream-rules.mjs')) {
  runCli();
}
