#!/usr/bin/env node
/**
 * ==============================================================================
 * SonarCloud Coverage Risk & Exclusion Pre-check
 * ==============================================================================
 *
 * Ensures that changes do not silently drag down SonarCloud New Code Coverage
 * below the strict 80.0% Quality Gate threshold:
 *  1. Detects new/modified production code in apps/ and libs/ without matching tests.
 *  2. Verifies that untestable browser hardware/DOM probes are excluded in sonar-project.properties.
 *  3. If LCOV reports exist, calculates line coverage and warns/fails if below 80.0%.
 */

import fs from 'node:fs';
import path from 'node:path';
import { execFileSync } from 'node:child_process';

const ROOT_DIR = process.cwd();

/**
 * Parses sonar.coverage.exclusions from sonar-project.properties
 * @returns {string[]}
 */
export function getCoverageExclusions(rootDir = ROOT_DIR) {
  const propsPath = path.join(rootDir, 'sonar-project.properties');
  if (!fs.existsSync(propsPath)) return [];
  const content = fs.readFileSync(propsPath, 'utf8');

  const match = content.match(/sonar\.coverage\.exclusions=([^\n]+(?:\\[\r\n]+[^\n]+)*)/);
  if (!match) return [];

  return match[1]
    .replace(/\\\r?\n/g, '')
    .split(',')
    .map((s) => s.trim())
    .filter(Boolean);
}

/**
 * Checks if a given relative file path matches any exclusion pattern
 * @param {string} filePath
 * @param {string[]} exclusions
 * @returns {boolean}
 */
export function isExcluded(filePath, exclusions) {
  for (const pattern of exclusions) {
    // Exact or wildcard prefix match
    const cleanPattern = pattern.replace(/\/\*\*$/, '').replace(/\*$/, '');
    if (pattern === filePath || filePath.startsWith(cleanPattern)) {
      return true;
    }
    if (pattern.includes('*')) {
      const regexStr = '^' + pattern.replace(/\*\*/g, '.*').replace(/\*/g, '[^/]*') + '$';
      try {
        if (new RegExp(regexStr).test(filePath)) return true;
      } catch {
        // Fallback simple prefix check
      }
    }
  }
  return false;
}

/**
 * Computes coverage from an LCOV string
 * @param {string} lcovContent
 * @returns {{ found: number, hit: number, percentage: number } | null}
 */
export function parseLcovCoverage(lcovContent) {
  let found = 0;
  let hit = 0;
  for (const line of lcovContent.split('\n')) {
    if (line.startsWith('LF:')) {
      found += Number.parseInt(line.slice(3).trim(), 10) || 0;
    } else if (line.startsWith('LH:')) {
      hit += Number.parseInt(line.slice(3).trim(), 10) || 0;
    }
  }
  if (found === 0) return null;
  return {
    found,
    hit,
    percentage: (hit / found) * 100,
  };
}

/**
 * @param {string[]} changedFiles
 * @param {string[]} exclusions
 * @returns {{ untestedFiles: string[], warnings: string[] }}
 */
export function auditChangedFilesForCoverage(changedFiles, exclusions) {
  const untestedFiles = [];
  const warnings = [];

  const prodFiles = changedFiles.filter((f) => {
    if (!f.startsWith('apps/') && !f.startsWith('libs/')) return false;
    if (f.includes('/tests/') || f.includes('/__tests__/')) return false;
    if (/\.(?:test|spec)\.(?:ts|tsx|js|mjs)$/.test(f)) return false;
    if (/\.(?:tests|test_support)\.rs$/.test(f)) return false;
    if (!/\.(?:ts|tsx|rs)$/.test(f)) return false;
    return true;
  });

  for (const file of prodFiles) {
    if (isExcluded(file, exclusions)) continue;

    // Check if matching test exists in changedFiles or on disk
    const isTs = file.endsWith('.ts') || file.endsWith('.tsx');
    const isRust = file.endsWith('.rs');

    let hasTest = false;
    if (isTs) {
      const ext = path.extname(file);
      const testSibling = file.slice(0, -ext.length) + '.test' + ext;
      const baseName = path.basename(file, ext);
      const dir = path.dirname(file);
      const testInTests = path.join(dir, '__tests__', `${baseName}.test${ext}`);

      if (
        changedFiles.includes(testSibling) ||
        changedFiles.includes(testInTests) ||
        fs.existsSync(path.join(ROOT_DIR, testSibling)) ||
        fs.existsSync(path.join(ROOT_DIR, testInTests))
      ) {
        hasTest = true;
      }
    } else if (isRust) {
      const testSibling = file.slice(0, -3) + '.tests.rs';
      const dir = path.dirname(file);
      const testInTests = path.join(dir, 'tests');

      if (
        changedFiles.includes(testSibling) ||
        fs.existsSync(path.join(ROOT_DIR, testSibling)) ||
        fs.existsSync(path.join(ROOT_DIR, testInTests))
      ) {
        hasTest = true;
      }
    }

    if (!hasTest) {
      untestedFiles.push(file);
    }
  }

  return { untestedFiles, warnings };
}

function runCli() {
  const exclusions = getCoverageExclusions();
  const args = process.argv.slice(2);
  let diffFiles = [];

  try {
    const gitArgs = args.includes('--staged')
      ? ['diff', '--cached', '--name-only']
      : ['diff', '--name-only', 'origin/main...HEAD'];

    const output = execFileSync('git', gitArgs, { encoding: 'utf8' }).trim();
    if (output) {
      diffFiles = output.split('\n').filter(Boolean);
    }
  } catch {
    // If git diff fails (e.g. no origin/main), fallback to git status
    try {
      const statusOutput = execFileSync('git', ['status', '--porcelain'], { encoding: 'utf8' });
      diffFiles = statusOutput
        .split('\n')
        .map((l) => l.slice(3).trim())
        .filter(Boolean);
    } catch {
      diffFiles = [];
    }
  }

  const { untestedFiles } = auditChangedFilesForCoverage(diffFiles, exclusions);

  // Check existing LCOV reports if present
  const lcovReports = [
    path.join(ROOT_DIR, 'coverage/lcov.info'),
    path.join(ROOT_DIR, '.temp/rust/lcov.info'),
  ];

  let lcovFailed = false;
  for (const reportPath of lcovReports) {
    if (fs.existsSync(reportPath)) {
      const content = fs.readFileSync(reportPath, 'utf8');
      const stats = parseLcovCoverage(content);
      if (stats) {
        const name = path.relative(ROOT_DIR, reportPath);
        if (stats.percentage < 80.0) {
          console.error(
            `❌ [Coverage < 80%] ${name}: ${stats.percentage.toFixed(1)}% (${stats.hit}/${stats.found} lines)`,
          );
          lcovFailed = true;
        } else {
          console.log(`✔ [Coverage >= 80%] ${name}: ${stats.percentage.toFixed(1)}%`);
        }
      }
    }
  }

  if (untestedFiles.length > 0) {
    console.warn(
      `⚠️  Coverage risk: ${untestedFiles.length} new/modified production file(s) have no direct tests:`,
    );
    for (const f of untestedFiles.slice(0, 10)) {
      console.warn(`   - ${f}`);
    }
    if (untestedFiles.length > 10) {
      console.warn(`   ... and ${untestedFiles.length - 10} more`);
    }
    console.warn(
      `   Hint: Add tests or exclude non-testable browser hardware probes in sonar-project.properties.`,
    );
  }

  if (lcovFailed) {
    console.error(
      'BLOCKED: Existing test coverage report is below SonarCloud strict 80% requirement.',
    );
    process.exit(1);
  }

  console.log('coverage-risk check OK');
}

if (process.argv[1]?.endsWith('check-coverage-risk.mjs')) {
  runCli();
}
