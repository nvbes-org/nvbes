import { spawnSync } from 'node:child_process';
import { appendFileSync } from 'node:fs';

const plan = JSON.parse(process.env.NVBES_CI_PLAN);
const lane = process.argv[2];
if (plan.version !== 1 || plan.candidate[lane] !== true)
  throw new Error('Lane not required by valid plan');

const cache = { hits: 0, total: 0 };
function run(binary, args, capture = false) {
  const result = spawnSync(binary, args, {
    stdio: capture ? 'pipe' : 'inherit',
    encoding: 'utf8',
    maxBuffer: 32 * 1024 * 1024,
  });
  if (capture) {
    process.stdout.write(result.stdout ?? '');
    process.stderr.write(result.stderr ?? '');
    const output = (result.stdout ?? '').replace(/\u001b\[[0-9;]*m/gu, '');
    const match = output.match(/Cache:\s+(\d+)\/(\d+) hit/u);
    if (match) {
      cache.hits += Number(match[1]);
      cache.total += Number(match[2]);
    }
  }
  if (result.status !== 0) throw new Error(`${binary} failed (${result.status})`);
}
function nx(target, projects, parallel = 4) {
  if (!projects.length) return;
  if (!projects.every((project) => /^[a-zA-Z0-9@/_.-]+$/u.test(project)))
    throw new Error('Invalid project name');
  run(
    'pnpm',
    [
      'exec',
      'nx',
      'run-many',
      '-t',
      target,
      `--projects=${projects.join(',')}`,
      `--parallel=${parallel}`,
      '--outputStyle=static',
    ],
    true,
  );
}
const started = Date.now();
function selected(target, projects, baseline, parallel = 4) {
  if (projects.some((project) => !baseline.includes(project)))
    throw new Error('Candidate exceeds baseline');
  nx(target, projects, parallel);
  if (plan.shadow) {
    // Each target has the same inputs in baseline and candidate; executing the
    // remaining projects proves the full envelope without repeating DB tests.
    nx(
      target,
      baseline.filter((project) => !projects.includes(project)),
      parallel,
    );
  }
}
switch (lane) {
  case 'typescript':
    for (const [target, projects] of Object.entries(plan.targets))
      selected(target, projects, plan.baseline.targets[target]);
    break;
  case 'database':
    selected('test:database', plan.databases, plan.baseline.databases, 1);
    break;
  case 'containers':
    selected('test:contract', plan.containers, plan.baseline.containers);
    break;
  case 'terraform':
    selected('terraform:validate', plan.terraform, plan.baseline.terraform, 3);
    selected('terraform:test', plan.terraform, plan.baseline.terraform, 1);
    if (
      (plan.shadow ? plan.baseline.terraform : plan.terraform).some((name) =>
        name.includes('email'),
      )
    )
      nx('test:contract', ['ci-contracts'], 1);
    break;
  case 'contracts':
    nx('test:ci-contract', ['ci-contracts'], 1);
    nx('test:lockfile-integration', ['ci-contracts'], 1);
    if (!plan.ciOnly) {
      run('pnpm', [
        'exec',
        'vp',
        'fmt',
        '--check',
        'package.json',
        'pnpm-workspace.yaml',
        'vite.config.ts',
        'tsconfig.base.json',
        'nx.json',
      ]);
      for (const check of [
        'check:structure',
        'check:nx-boundaries',
        'check:secrets',
        'check:web-runtime-guardrails',
        'check:ajax-security',
      ])
        run('pnpm', [check]);
    }
    break;
  default:
    throw new Error(`Unknown lane ${lane}`);
}
const metric = {
  lane,
  durationMs: Date.now() - started,
  head: plan.head,
  fallbackFull: plan.fallbackFull,
  shadow: plan.shadow,
  nxCache: cache.total ? cache : null,
};
console.log(`CI_METRIC ${JSON.stringify(metric)}`);
appendFileSync(process.env.GITHUB_STEP_SUMMARY, `\nCI_METRIC ${JSON.stringify(metric)}\n`);
