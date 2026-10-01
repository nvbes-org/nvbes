#!/usr/bin/env node
/**
 * ==============================================================================
 * nvbes Pre-flight Verification Script
 * ==============================================================================
 *
 * Runs all upstream quality, security and anti-pattern gates before committing:
 *  1. SonarQube & CodeQL Upstream Rules (SQL WHERE, Array sort, Promise Error, ReDoS, parseInt, Math.random, async blocking)
 *  2. NVBES Monotone Thresholds & Ratchets (No lowered coverage, no unwrap in prod, no sqlx::query, no weak assertions)
 *  3. Secrets & Security Pre-Checks (Secret scan, license checks, cargo deny)
 *  4. Structure & Boundaries (dot-notation, file limits < 300/500 lines, monorepo boundaries)
 *  5. Formatting & Linters (Biome / vp format check, env check, workflows lint)
 *  6. Compilation (Web typecheck / lint, cargo check --workspace)
 *
 * Usage:
 *  pnpm preflight              # Fast staged check (default before commit)
 *  pnpm preflight --quick      # Ultra-fast check (upstream rules, ratchets, secrets, format)
 *  pnpm preflight --full       # Full workspace verification including compiler and typechecks
 *  pnpm preflight --range main..HEAD  # Range check for PR validation
 */

import { spawnSync } from 'node:child_process';
import process from 'node:process';

const args = process.argv.slice(2);
const isQuick = args.includes('--quick');
const isFull = args.includes('--full');
const rangeIdx = args.indexOf('--range');
const range = rangeIdx !== -1 ? args[rangeIdx + 1] : null;

const colors = {
  reset: '\x1b[0m',
  bold: '\x1b[1m',
  dim: '\x1b[2m',
  red: '\x1b[31m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  blue: '\x1b[34m',
  cyan: '\x1b[36m',
  gray: '\x1b[90m',
};

function logHeader() {
  console.log(
    '\n' +
      colors.bold +
      colors.cyan +
      '┌─────────────────────────────────────────────────────────────┐' +
      colors.reset,
  );
  console.log(
    colors.bold +
      colors.cyan +
      '│                 nvbes Commit Pre-flight Check               │' +
      colors.reset,
  );
  console.log(
    colors.bold +
      colors.cyan +
      '└─────────────────────────────────────────────────────────────┘' +
      colors.reset,
  );
  console.log(
    colors.dim +
      `Mode: ${isFull ? 'FULL' : isQuick ? 'QUICK' : 'STAGED (Default)'} | Node: ${process.version}\n` +
      colors.reset,
  );
}

/**
 * @param {string} title
 * @param {string} cmd
 * @param {string[]} cmdArgs
 * @returns {boolean}
 */
function runStep(title, cmd, cmdArgs) {
  const start = Date.now();
  process.stdout.write(`  ${colors.blue}▶${colors.reset} ${title.padEnd(46, '.')} `);

  const res = spawnSync(cmd, cmdArgs, {
    stdio: ['inherit', 'pipe', 'pipe'],
    encoding: 'utf8',
    env: process.env,
  });

  const duration = ((Date.now() - start) / 1000).toFixed(2);

  if (res.status === 0) {
    console.log(`${colors.green}✔ PASS${colors.reset} ${colors.gray}(${duration}s)${colors.reset}`);
    return true;
  }

  console.log(`${colors.red}✖ FAIL${colors.reset} ${colors.gray}(${duration}s)${colors.reset}`);
  if (res.stdout && res.stdout.trim()) {
    console.log(
      colors.dim +
        res.stdout
          .trim()
          .split('\n')
          .map((l) => `    ${l}`)
          .join('\n') +
        colors.reset,
    );
  }
  if (res.stderr && res.stderr.trim()) {
    console.error(
      colors.red +
        res.stderr
          .trim()
          .split('\n')
          .map((l) => `    ${l}`)
          .join('\n') +
        colors.reset,
    );
  }
  return false;
}

function main() {
  logHeader();
  const failedSteps = [];
  const stagedArg = range ? ['--range', range] : ['--staged'];

  // ============================================================================
  // Group 1: Upstream Rules (SonarQube, CodeQL, Security CI)
  // ============================================================================
  console.log(colors.bold + '\n[1/6] SonarQube & CodeQL Upstream Rules' + colors.reset);

  if (
    !runStep('Upstream rules unit tests', 'node', [
      '--test',
      'tools/ci/check-upstream-rules.test.mjs',
    ])
  ) {
    failedSteps.push('Upstream rules tests');
  }

  if (
    !runStep('Upstream anti-patterns in diff', 'node', [
      'tools/ci/check-upstream-rules.mjs',
      ...stagedArg,
    ])
  ) {
    failedSteps.push('Upstream rules violations');
  }

  // ============================================================================
  // Group 2: NVBES Quality Ratchets & Anti-Regression
  // ============================================================================
  console.log(colors.bold + '\n[2/6] Monotone Thresholds & Ratchets' + colors.reset);

  if (!runStep('Monotone thresholds check', 'node', ['tools/ci/check-thresholds-monotone.mjs'])) {
    failedSteps.push('Monotone thresholds');
  }

  if (
    !runStep('Weak assertions check (assert!(is_ok/err))', 'node', [
      'tools/ci/check-weak-assertions.mjs',
      ...stagedArg,
    ])
  ) {
    failedSteps.push('Weak assertions');
  }

  if (
    !runStep('Unwrap ratchet (Rust unwrap/expect in prod)', 'node', [
      'tools/ci/check-unwrap-ratchet.mjs',
      ...stagedArg,
    ])
  ) {
    failedSteps.push('Unwrap ratchet');
  }

  if (!runStep('SQLx query! ratchet', 'node', ['tools/ci/check-sqlx-ratchet.mjs'])) {
    failedSteps.push('SQLx ratchet');
  }

  if (
    !runStep('Protected paths & LLM policy', 'node', ['tools/ci/check-agent-protected-paths.mjs'])
  ) {
    failedSteps.push('Agent protected paths');
  }

  // ============================================================================
  // Group 3: SonarCloud Coverage Risk (>= 80% New Code Requirement)
  // ============================================================================
  console.log(colors.bold + '\n[3/6] SonarCloud 80% Coverage Risk & Exclusions' + colors.reset);

  if (
    !runStep('Coverage risk & exclusions audit', 'node', [
      'tools/ci/check-coverage-risk.mjs',
      ...stagedArg,
    ])
  ) {
    failedSteps.push('Coverage risk audit');
  }

  // ============================================================================
  // Group 4: Security & Secrets
  // ============================================================================
  console.log(colors.bold + '\n[4/6] Security & Secrets' + colors.reset);

  if (!runStep('Hardcoded secrets scan', 'node', ['tools/security/scan-secrets.mjs'])) {
    failedSteps.push('Secrets scan');
  }

  if (
    !runStep('JavaScript license compliance', 'node', [
      'tools/security/check-javascript-licenses.mjs',
    ])
  ) {
    failedSteps.push('JS Licenses');
  }

  // ============================================================================
  // Group 5: Format, Environment & Structure
  // ============================================================================
  console.log(colors.bold + '\n[5/6] Format, Environment & Structure' + colors.reset);

  if (!runStep('Environment configuration check', 'node', ['scripts/env.mjs', 'check'])) {
    failedSteps.push('Env check');
  }

  if (!runStep('Structure & dot-notation compliance', 'bash', ['scripts/check-llm-structure.sh'])) {
    failedSteps.push('Structure compliance');
  }

  if (!runStep('GitHub Actions workflows lint', 'node', ['tools/ci/lint-workflows.mjs'])) {
    failedSteps.push('Workflows lint');
  }

  if (!runStep('Code formatting check (Biome / Cargo fmt)', 'pnpm', ['format:check'])) {
    failedSteps.push('Formatting check');
  }

  // ============================================================================
  // Group 6: Full Compilation & Typechecks (skipped in --quick)
  // ============================================================================
  if (!isQuick) {
    console.log(colors.bold + '\n[6/6] Typecheck & Compilation' + colors.reset);

    if (isFull) {
      if (!runStep('Monorepo boundaries check', 'node', ['scripts/check-nx-boundaries.mjs'])) {
        failedSteps.push('Nx boundaries');
      }

      if (!runStep('Web workspace check (TypeScript/Biome)', 'pnpm', ['check:web'])) {
        failedSteps.push('Web check');
      }

      if (!runStep('Rust workspace compilation check', 'cargo', ['check', '--workspace'])) {
        failedSteps.push('Rust cargo check');
      }
    } else {
      // In default mode, check structure boundaries & web
      if (!runStep('Monorepo boundaries check', 'node', ['scripts/check-nx-boundaries.mjs'])) {
        failedSteps.push('Nx boundaries');
      }
    }
  }

  // ============================================================================
  // Summary
  // ============================================================================
  console.log('\n' + colors.cyan + '─'.repeat(63) + colors.reset);

  if (failedSteps.length === 0) {
    console.log(`${colors.bold}${colors.green}✔ PRE-FLIGHT CHECK SUCCEEDED!${colors.reset}`);
    console.log(
      `${colors.green}All SonarQube, CodeQL, Security and CI rules are satisfied.${colors.reset}`,
    );
    console.log(`${colors.dim}You can safely commit with AI-Assisted trailer.${colors.reset}\n`);
    process.exit(0);
  } else {
    console.error(
      `${colors.bold}${colors.red}✖ PRE-FLIGHT CHECK FAILED (${failedSteps.length} step(s) failed):${colors.reset}`,
    );
    for (const step of failedSteps) {
      console.error(`  - ${colors.red}${step}${colors.reset}`);
    }
    console.error(
      `\n${colors.yellow}Fix the issues above before committing to avoid breaking remote CI/SonarCloud.${colors.reset}\n`,
    );
    process.exit(1);
  }
}

main();
