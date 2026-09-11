import { fallbackPlan } from './runner-fallback.core.mjs';

const repository = process.env.GITHUB_REPOSITORY;
const runId = process.env.GITHUB_RUN_ID;
if (!/^[\w.-]+\/[\w.-]+$/u.test(repository ?? '') || !/^\d+$/u.test(runId ?? ''))
  throw new Error('Invalid workflow context');
const token = process.env.GITHUB_TOKEN;
if (!token) throw new Error('Missing GitHub token');
const base = `https://api.github.com/repos/${repository}`;
async function api(resource, body) {
  const response = await fetch(`${base}/${resource}`, {
    method: body ? 'POST' : 'GET',
    headers: {
      Authorization: `Bearer ${token}`,
      Accept: 'application/vnd.github+json',
      'X-GitHub-Api-Version': '2022-11-28',
    },
    ...(body ? { body: JSON.stringify(body) } : {}),
    signal: AbortSignal.timeout(20_000),
  });
  if (!response.ok) throw new Error(`GitHub API failed: ${response.status}`);
  return response.status === 204 ? undefined : response.json();
}

const run = await api(`actions/runs/${runId}`);
if (!['.github/workflows/ci.yml', '.github/workflows/v1-testing.yml'].includes(run.path))
  throw new Error('Unexpected workflow');
const workflow = run.path.split('/').at(-1);
const response = await api(`actions/runs/${runId}/jobs?per_page=100`);
if (response.total_count > 100) throw new Error('Job inventory exceeds fallback bound');
const failed = response.jobs.filter(
  (job) => job.conclusion === 'failure' && !['ci-gate', 'local-fallback'].includes(job.name),
);
if (
  failed.length === 0 ||
  failed.some(
    (job) => job.steps?.length !== 0 || job.runner_name || !job.labels?.includes('ubuntu-latest'),
  )
) {
  console.log('No unstarted hosted failure: no fallback');
  process.exit(0);
}
const annotations = {};
for (const job of response.jobs.filter((entry) => entry.conclusion === 'failure')) {
  annotations[job.id] = await api(`check-runs/${job.id}/annotations?per_page=100`);
}
const [branch, commit, actor, history] = await Promise.all([
  api(`git/ref/heads/${encodeURIComponent(run.head_branch)}`),
  api(`commits/${run.head_sha}`),
  api(`collaborators/${encodeURIComponent(run.actor.login)}/permission`),
  api(`actions/workflows/${workflow}/runs?event=workflow_dispatch&per_page=100`),
]);
const plan = fallbackPlan({
  run,
  jobs: response.jobs,
  annotations,
  repository,
  branchSha: branch.object.sha,
  commit,
  permission: actor.permission,
  existing: history.workflow_runs,
});
// The workflow checks this SHA again before allocating any local execution lane.
await api(`actions/workflows/${workflow}/dispatches`, plan);
console.log(`Dispatched local fallback for run ${runId} at ${run.head_sha}`);
