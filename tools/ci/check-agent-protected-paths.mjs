#!/usr/bin/env node
/**
 * Workflow 100% LLM avec reprise humaine optionnelle.
 *
 * - Auteur agent => trailer AI-Assisted obligatoire.
 * - Commit agent touchant une zone protégée => trailer
 *   Human-Review-Required: protected-paths obligatoire (le commit EST autorisé).
 * - La reprise humaine est la revue CODEOWNERS / job CI human-gate, pas un
 *   hard-block local (NVBES_HUMAN_GATE reste un contournement d'urgence).
 * - Les seuils monotones restent un hard-block séparé (anti-Goodhart).
 */
import { execFileSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';

export const PROTECTED_PATH_PREFIXES = [
  'lefthook.yml',
  'deny.toml',
  '.github/',
  'tools/ci/',
  'tools/security/',
  'docs/testing/rust-coverage-thresholds.json',
  'docs/testing/rust-mutation-thresholds.json',
  'docs/testing/rust-condition-thresholds.json',
  'commitlint.config.cjs',
  'AGENTS.md',
];

export const AGENT_AUTHOR_PATTERN =
  /^(Cursor Agent|Synthetic Test|Codex|Claude|GPT-|OpenAI|Gemini|Composer)/iu;

export const HUMAN_REVIEW_TRAILER = 'Human-Review-Required: protected-paths';

/**
 * @param {string} path
 * @returns {boolean}
 */
export function isProtectedPath(path) {
  const normalized = path.replaceAll('\\', '/');
  return PROTECTED_PATH_PREFIXES.some(
    (prefix) => normalized === prefix || normalized.startsWith(prefix),
  );
}

/**
 * @param {string} message
 * @returns {boolean}
 */
export function hasAiAssistedTrailer(message) {
  return /^AI-Assisted:\s+\S+/imu.test(message);
}

/**
 * @param {string} message
 * @returns {boolean}
 */
export function isValidAiAssistedTrailer(message) {
  if (!/^AI-Assisted:/imu.test(message)) return true;
  return /^AI-Assisted:\s+\S.+$/imu.test(message);
}

/**
 * @param {string} message
 * @returns {boolean}
 */
export function hasHumanReviewRequiredTrailer(message) {
  return /^Human-Review-Required:\s*protected-paths\s*$/imu.test(message);
}

/**
 * @param {{ authorName?: string, message?: string }} identity
 * @returns {boolean}
 */
export function isAgentCommit(identity) {
  if (identity.message && hasAiAssistedTrailer(identity.message)) return true;
  if (identity.authorName && AGENT_AUTHOR_PATTERN.test(identity.authorName.trim())) return true;
  return false;
}

/**
 * @param {string[]} paths
 * @param {{
 *   isAgent: boolean,
 *   hasAiAssisted: boolean,
 *   hasHumanReview: boolean,
 *   humanGate?: boolean,
 * }} options
 * @returns {{
 *   ok: boolean,
 *   protectedTouched: string[],
 *   reasons: string[],
 *   requiresHumanReview: boolean,
 * }}
 */
export function evaluateLlmCommitPolicy(paths, options) {
  const protectedTouched = [...new Set(paths.filter(isProtectedPath))];
  /** @type {string[]} */
  const reasons = [];

  if (options.humanGate) {
    return { ok: true, protectedTouched, reasons, requiresHumanReview: false };
  }

  if (options.isAgent && !options.hasAiAssisted) {
    reasons.push('agent author requires trailer "AI-Assisted: <agent-or-model>"');
  }

  const requiresHumanReview = options.isAgent && protectedTouched.length > 0;
  if (requiresHumanReview && !options.hasHumanReview) {
    reasons.push(
      `protected paths modified by LLM require trailer "${HUMAN_REVIEW_TRAILER}" (CODEOWNERS CI gate)`,
    );
  }

  return {
    ok: reasons.length === 0,
    protectedTouched,
    reasons,
    requiresHumanReview,
  };
}

/** @deprecated use evaluateLlmCommitPolicy */
export function evaluateProtectedPaths(paths, options) {
  const result = evaluateLlmCommitPolicy(paths, {
    isAgent: options.isAgent,
    hasAiAssisted: true,
    hasHumanReview: Boolean(options.humanGate),
    humanGate: options.humanGate,
  });
  return { ok: result.ok, blocked: result.protectedTouched };
}

/**
 * @returns {string[]}
 */
export function stagedPaths() {
  const out = execFileSync('git', ['diff', '--cached', '--name-only', '-z'], {
    encoding: 'utf8',
  });
  return out.split('\0').filter(Boolean);
}

/**
 * @returns {{ authorName: string, message: string }}
 */
export function commitIdentityFromEnv() {
  let authorName = process.env.GIT_AUTHOR_NAME || '';
  if (!authorName) {
    try {
      authorName = execFileSync('git', ['config', 'user.name'], { encoding: 'utf8' }).trim();
    } catch {
      authorName = '';
    }
  }
  let message = '';
  const editPath = process.argv.includes('--edit')
    ? process.argv[process.argv.indexOf('--edit') + 1]
    : process.env.LEFTHOOK_COMMIT_MSG || '.git/COMMIT_EDITMSG';
  try {
    message = readFileSync(editPath, 'utf8');
  } catch {
    message = '';
  }
  return { authorName, message };
}

/**
 * @param {{ paths: string[], identity: { authorName: string, message: string } }} input
 */
export function writeHumanReviewReceipt(input) {
  const out = resolve('.temp/llm/human-review-required.json');
  mkdirSync(dirname(out), { recursive: true });
  writeFileSync(
    out,
    `${JSON.stringify(
      {
        schemaVersion: 1,
        required: true,
        reason: 'protected-paths',
        trailer: HUMAN_REVIEW_TRAILER,
        authorName: input.identity.authorName,
        paths: input.paths.filter(isProtectedPath),
        generatedAt: new Date().toISOString(),
      },
      null,
      2,
    )}\n`,
  );
  return out;
}

const isCli = process.argv[1]?.endsWith('check-agent-protected-paths.mjs');
if (isCli) {
  if (process.env.NVBES_HUMAN_GATE === '1') {
    console.log('llm-commit-policy OK (NVBES_HUMAN_GATE=1 emergency override)');
    process.exit(0);
  }
  const identity = commitIdentityFromEnv();
  if (!isValidAiAssistedTrailer(identity.message)) {
    console.error(
      'BLOCKED AI-Assisted trailer: expected "AI-Assisted: <agent-or-model>" on its own line',
    );
    process.exit(1);
  }
  const agent = isAgentCommit(identity);
  const paths = stagedPaths();
  const result = evaluateLlmCommitPolicy(paths, {
    isAgent: agent,
    hasAiAssisted: hasAiAssistedTrailer(identity.message),
    hasHumanReview: hasHumanReviewRequiredTrailer(identity.message),
  });
  if (!result.ok) {
    console.error('BLOCKED llm-commit-policy:');
    for (const reason of result.reasons) console.error(`- ${reason}`);
    if (result.protectedTouched.length) {
      console.error('Protected paths:');
      for (const path of result.protectedTouched) console.error(`- ${path}`);
    }
    console.error(
      `LLM may modify gates; add trailers then rely on CODEOWNERS. Example:\n\nAI-Assisted: Cursor\n${HUMAN_REVIEW_TRAILER}\n`,
    );
    process.exit(1);
  }
  if (result.requiresHumanReview) {
    const receipt = writeHumanReviewReceipt({ paths, identity });
    console.log(
      `llm-commit-policy OK (agent + protected paths; human reprise queued → ${receipt})`,
    );
  } else if (agent) {
    console.log(`llm-commit-policy OK (${paths.length} staged, agent commit, no protected)`);
  } else {
    console.log(`llm-commit-policy OK (${paths.length} staged, human commit)`);
  }
}
