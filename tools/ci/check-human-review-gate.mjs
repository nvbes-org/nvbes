#!/usr/bin/env node
/**
 * Gate CI de reprise humaine pour PRs qui touchent les zones protégées.
 *
 * - Si le diff ne touche aucune zone protégée : OK.
 * - Sinon : exige au moins une review APPROVED d'un reviewer humain
 *   (pas un bot) listé dans CODEOWNERS / NVBES_HUMAN_REVIEWERS.
 * - Contournement d'urgence : label `human-gate-approved` ou
 *   NVBES_HUMAN_GATE=1.
 */
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { isProtectedPath } from './check-agent-protected-paths.mjs';

export const BOT_LOGIN_PATTERN = /\[bot\]$/iu;

/**
 * @param {string} codeownersText
 * @returns {string[]}
 */
export function parseCodeownersLogins(codeownersText) {
  /** @type {Set<string>} */
  const logins = new Set();
  for (const raw of codeownersText.split('\n')) {
    const line = raw.trim();
    if (!line || line.startsWith('#')) continue;
    for (const token of line.split(/\s+/u).slice(1)) {
      if (token.startsWith('@') && !token.includes('/')) {
        logins.add(token.slice(1).toLowerCase());
      }
    }
  }
  return [...logins];
}

/**
 * @param {string[]} changedPaths
 * @returns {string[]}
 */
export function protectedPathsInDiff(changedPaths) {
  return [...new Set(changedPaths.filter(isProtectedPath))];
}

/**
 * @param {{
 *   protectedPaths: string[],
 *   reviews: { user: string, state: string }[],
 *   labels?: string[],
 *   humanReviewers: string[],
 *   humanGate?: boolean,
 * }} input
 */
export function evaluateHumanReviewGate(input) {
  if (input.humanGate) return { ok: true, reason: 'NVBES_HUMAN_GATE' };
  if (input.protectedPaths.length === 0) {
    return { ok: true, reason: 'no-protected-paths' };
  }
  if ((input.labels ?? []).map((l) => l.toLowerCase()).includes('human-gate-approved')) {
    return { ok: true, reason: 'label:human-gate-approved' };
  }
  const reviewers = new Set(input.humanReviewers.map((r) => r.toLowerCase()));
  const approved = input.reviews.filter(
    (review) =>
      review.state.toUpperCase() === 'APPROVED' &&
      !BOT_LOGIN_PATTERN.test(review.user) &&
      reviewers.has(review.user.toLowerCase()),
  );
  if (approved.length > 0) {
    return { ok: true, reason: `approved-by:${approved.map((r) => r.user).join(',')}` };
  }
  return {
    ok: false,
    reason: 'waiting-for-codeowners-approval',
    protectedPaths: input.protectedPaths,
  };
}

/**
 * @param {string} baseRef
 * @param {string} headRef
 * @returns {string[]}
 */
export function changedPathsInRange(baseRef, headRef) {
  const out = execFileSync('git', ['diff', '--name-only', `${baseRef}...${headRef}`], {
    encoding: 'utf8',
  });
  return out
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean);
}

function loadHumanReviewers() {
  if (process.env.NVBES_HUMAN_REVIEWERS) {
    return process.env.NVBES_HUMAN_REVIEWERS.split(',')
      .map((s) => s.trim())
      .filter(Boolean);
  }
  try {
    return parseCodeownersLogins(readFileSync('.github/CODEOWNERS', 'utf8'));
  } catch {
    return [];
  }
}

/**
 * @returns {{ user: string, state: string }[]}
 */
function loadPullRequestReviews() {
  if (process.env.NVBES_PR_REVIEWS_JSON) {
    return JSON.parse(process.env.NVBES_PR_REVIEWS_JSON);
  }
  const repo = process.env.GITHUB_REPOSITORY;
  const pr = process.env.NVBES_PR_NUMBER || process.env.GITHUB_PR_NUMBER;
  if (!repo || !pr) return [];
  try {
    const raw = execFileSync(
      'gh',
      ['api', `repos/${repo}/pulls/${pr}/reviews`, '--jq', '[.[]|{user:.user.login,state:.state}]'],
      { encoding: 'utf8' },
    );
    return JSON.parse(raw);
  } catch {
    return [];
  }
}

/**
 * @returns {string[]}
 */
function loadPullRequestLabels() {
  if (process.env.NVBES_PR_LABELS) {
    return process.env.NVBES_PR_LABELS.split(',')
      .map((s) => s.trim())
      .filter(Boolean);
  }
  return [];
}

const isCli = process.argv[1]?.endsWith('check-human-review-gate.mjs');
if (isCli) {
  const baseIdx = process.argv.indexOf('--base');
  const headIdx = process.argv.indexOf('--head');
  const baseRef = baseIdx >= 0 ? process.argv[baseIdx + 1] : 'origin/main';
  const headRef = headIdx >= 0 ? process.argv[headIdx + 1] : 'HEAD';
  const paths = changedPathsInRange(baseRef, headRef);
  const protectedPaths = protectedPathsInDiff(paths);
  const result = evaluateHumanReviewGate({
    protectedPaths,
    reviews: loadPullRequestReviews(),
    labels: loadPullRequestLabels(),
    humanReviewers: loadHumanReviewers(),
    humanGate: process.env.NVBES_HUMAN_GATE === '1',
  });
  if (!result.ok) {
    console.error('BLOCKED human-review-gate: protected paths changed by LLM workflow');
    for (const path of result.protectedPaths ?? []) console.error(`- ${path}`);
    console.error(
      'Reprise humaine: approve the PR as CODEOWNER, or apply label human-gate-approved.',
    );
    process.exit(1);
  }
  console.log(`human-review-gate OK (${result.reason})`);
}
