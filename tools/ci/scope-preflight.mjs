import { execFileSync, spawnSync } from 'node:child_process';
import { appendFileSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { resolveScopeBase } from './scope-base.mjs';
import { classifyPaths } from './scope-paths.mjs';

const git = (...args) => execFileSync('git', args, { encoding: 'utf8' }).trim();
const event = JSON.parse(readFileSync(process.env.GITHUB_EVENT_PATH, 'utf8'));
const startedAt = Date.now();
const head = process.env.GITHUB_SHA;
const repository = process.env.GITHUB_REPOSITORY;
async function* runs(branch) {
  for (let page = 1; page <= 10; page++) {
    const query = new URLSearchParams({
      branch,
      event: 'push',
      status: 'success',
      per_page: '100',
      page: String(page),
    });
    const response = await fetch(
      `https://api.github.com/repos/${repository}/actions/workflows/ci.yml/runs?${query}`,
      {
        headers: {
          Authorization: `Bearer ${process.env.GITHUB_TOKEN}`,
          Accept: 'application/vnd.github+json',
        },
        signal: AbortSignal.timeout(10_000),
      },
    );
    if (!response.ok) throw new Error(`GitHub HTTP ${response.status}`);
    const body = await response.json();
    yield* body.workflow_runs;
    if (body.workflow_runs.length < 100) return;
  }
}
const base = await resolveScopeBase(
  {
    event,
    head,
    repository,
    branch: process.env.GITHUB_REF_NAME,
    explicitBase: process.env.NVBES_CI_BASE_SHA,
    runId: process.env.GITHUB_RUN_ID,
  },
  {
    exists: (sha) => spawnSync('git', ['cat-file', '-e', `${sha}^{commit}`]).status === 0,
    ancestor: (a, b) => spawnSync('git', ['merge-base', '--is-ancestor', a, b]).status === 0,
    runs,
  },
);
// Disable rename detection so both old and new owners are validated.
const paths = base.fallback
  ? []
  : git('diff', '--no-renames', '--name-only', '-z', base.base, head).split('\0').filter(Boolean);
const classification = classifyPaths(paths, base.fallback);
const preflight = { ...base, ...classification, head, paths, startedAt };
mkdirSync('.nx/ci', { recursive: true });
writeFileSync('.nx/ci/preflight.json', JSON.stringify(preflight));
appendFileSync(process.env.GITHUB_OUTPUT, `graph-required=${classification.graphRequired}\n`);
