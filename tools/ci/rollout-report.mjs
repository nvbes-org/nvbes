import { execFileSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { summarizeRollout } from './rollout-report.core.mjs';

const repository = process.env.GITHUB_REPOSITORY || 'nvbes-org/nvbes';
const limit = Number(process.argv[2] ?? 100);
if (!Number.isInteger(limit) || limit < 1 || limit > 100)
  throw new Error('Run count must be between 1 and 100');
const gh = (...args) =>
  execFileSync('gh', args, { encoding: 'utf8', maxBuffer: 100 * 1024 * 1024 });
const listed = JSON.parse(
  gh('api', `repos/${repository}/actions/workflows/ci.yml/runs?per_page=${limit}`),
);
const runs = [];
for (const run of listed.workflow_runs.filter((run) => run.status === 'completed')) {
  const pages = JSON.parse(
    gh(
      'api',
      '--paginate',
      '--slurp',
      `repos/${repository}/actions/runs/${run.id}/jobs?per_page=100`,
    ),
  );
  const logs = gh('run', 'view', String(run.id), '--repo', repository, '--log');
  const evidence = [...logs.matchAll(/CI_RUST_EVIDENCE (\{[^\r\n]+\})/gu)].flatMap((match) => {
    const row = JSON.parse(match[1]);
    const linkedPr = run.pull_requests?.some((pr) => String(pr.number) === String(row.pr));
    return String(row.runId) === String(run.id) &&
      row.sourceHead === run.head_sha &&
      (run.event !== 'pull_request' || linkedPr)
      ? [{ ...row, timestamp: run.created_at, event: run.event }]
      : [];
  });
  const metrics = [...logs.matchAll(/CI_METRIC (\{[^\r\n]+\})/gu)].map((match) =>
    JSON.parse(match[1]),
  );
  runs.push({ ...run, jobs: pages.flatMap((page) => page.jobs), evidence, metrics });
}
const report = {
  generatedAt: new Date().toISOString(),
  repository,
  ...summarizeRollout(runs),
  observations: runs.flatMap((run) => run.evidence),
  runUrls: runs.map((run) => run.html_url),
};
mkdirSync('.nx/ci', { recursive: true });
writeFileSync('.nx/ci/rollout-report.json', JSON.stringify(report, null, 2));
console.log(JSON.stringify(report, null, 2));
