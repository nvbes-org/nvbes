import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';

const repository = process.env.GITHUB_REPOSITORY || 'nvbes-org/nvbes';
const apply = process.argv.includes('--apply');
const api = (path, ...args) =>
  JSON.parse(
    execFileSync('gh', ['api', `repos/${repository}/${path}`, ...args], { encoding: 'utf8' }),
  );
// Check account capabilities before proposing any policy change.
const rulesets = api('rulesets');
const desired = JSON.parse(readFileSync('.github/rulesets/ci-gate-policy.json', 'utf8'));
const existing = rulesets.find((rule) => rule.name === desired.name);
const runs = api(
  'actions/workflows/ci.yml/runs?branch=main&event=push&status=success&per_page=100',
).workflow_runs;
let proof;
for (const run of runs) {
  if (run.repository?.full_name !== repository || run.path !== '.github/workflows/ci.yml') continue;
  const jobs = api(`actions/runs/${run.id}/jobs?per_page=100`).jobs;
  if (jobs.some((job) => job.name === 'ci-gate' && job.conclusion === 'success')) {
    proof = run;
    break;
  }
}
if (!proof) throw new Error('ci-gate must first succeed in a main push CI; policy not changed');
console.log(
  JSON.stringify(
    { proof: proof.html_url, action: existing ? 'update' : 'create', policy: desired },
    null,
    2,
  ),
);
if (apply) {
  execFileSync(
    'gh',
    [
      'api',
      `repos/${repository}/rulesets${existing ? `/${existing.id}` : ''}`,
      '--method',
      existing ? 'PUT' : 'POST',
      '--input',
      '-',
    ],
    { input: JSON.stringify(desired), stdio: ['pipe', 'inherit', 'inherit'] },
  );
}
