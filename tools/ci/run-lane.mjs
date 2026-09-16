import { spawnSync } from 'node:child_process';
import { appendFileSync, mkdirSync } from 'node:fs';
import {
  checkOomExit,
  recommendedParallelism,
  startMemoryWatchdog,
  stopMemoryWatchdog,
} from './memory-guard.mjs';

startMemoryWatchdog({ intervalMs: 2000, thresholdMb: 500 });

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
    const ansiPattern = new RegExp(`${String.fromCharCode(27)}\\[[0-9;]*m`, 'gu');
    const output = (result.stdout ?? '').replace(ansiPattern, '');
    const match = output.match(/Cache:\s+(\d+)\/(\d+) hit/u);
    if (match) {
      cache.hits += Number(match[1]);
      cache.total += Number(match[2]);
    }
  }
  if (result.status !== 0) {
    checkOomExit(result.status, result.signal, `${binary} ${args.join(' ')}`);
    throw new Error(`${binary} failed (${result.status})`);
  }
}
function nx(target, projects, parallel = 4) {
  if (!projects.length) return;
  if (!projects.every((project) => /^[a-zA-Z0-9@/_.-]+$/u.test(project)))
    throw new Error('Invalid project name');
  const targetArgs = Array.isArray(target) ? target.flatMap((t) => ['-t', t]) : ['-t', target];
  run(
    'pnpm',
    [
      'exec',
      'nx',
      'run-many',
      ...targetArgs,
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
  case 'typescript': {
    const tsParallel = recommendedParallelism({ maxParallel: 4, memoryPerWorkerMb: 1536 });
    for (const [target, projects] of Object.entries(plan.targets)) {
      const targetParallel = target === 'typecheck' ? Math.min(tsParallel, 2) : tsParallel;
      selected(target, projects, plan.baseline.targets[target], targetParallel);
    }
    break;
  }
  case 'database': {
    const dbParallel = recommendedParallelism({ maxParallel: 4, memoryPerWorkerMb: 1024 });
    const targetProjects = plan.shadow ? plan.baseline.databases : plan.databases;
    const workspaceMap = {
      'account-service': 'nvbes-account-service',
      'billing-service': 'nvbes-billing-service',
      'identity-service': 'nvbes-identity-service',
      'email-worker': 'nvbes-email-worker',
      'trust-risk-service': 'nvbes-trust-risk-service',
      'platform-operations-service': 'nvbes-platform',
    };
    const workspacePkgs = targetProjects.map((project) => workspaceMap[project]).filter(Boolean);
    if (workspacePkgs.length > 0) {
      run('cargo', [
        'test',
        '--locked',
        '--features',
        'database-tests',
        '--no-run',
        ...workspacePkgs.flatMap((pkg) => ['--package', pkg]),
      ]);
    }
    selected('test:database', plan.databases, plan.baseline.databases, dbParallel);
    break;
  }
  case 'containers':
    selected('test:contract', plan.containers, plan.baseline.containers);
    break;
  case 'terraform':
    if (process.env.TF_PLUGIN_CACHE_DIR)
      mkdirSync(process.env.TF_PLUGIN_CACHE_DIR, { recursive: true });
    // Terraform's shared provider cache does not support concurrent installers.
    selected('terraform:validate', plan.terraform, plan.baseline.terraform, 1);
    selected('terraform:test', plan.terraform, plan.baseline.terraform, 1);
    if (
      (plan.shadow ? plan.baseline.terraform : plan.terraform).some((name) =>
        name.includes('email'),
      )
    )
      nx('test:contract', ['ci-contracts'], 1);
    break;
  case 'contracts':
    nx(['test:ci-contract', 'test:lockfile-integration'], ['ci-contracts'], 2);
    if (!plan.ciOnly) {
      run('pnpm', ['check:contracts']);
      run('pnpm', [
        'exec',
        'buf',
        'breaking',
        'contracts/protobuf',
        '--against',
        `.git#commit=${plan.base},subdir=contracts/protobuf`,
      ]);
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
        'check:product-boundaries',
        'check:secrets',
      ])
        run('pnpm', [check]);
    }
    break;
  default:
    throw new Error(`Unknown lane ${lane}`);
}
const memoryStats = stopMemoryWatchdog();
const metric = {
  lane,
  durationMs: Date.now() - started,
  head: plan.head,
  fallbackFull: plan.fallbackFull,
  shadow: plan.shadow,
  nxCache: cache.total ? cache : null,
  minMemoryAvailableMb: Number.isFinite(memoryStats.minAvailableMbSeen)
    ? memoryStats.minAvailableMbSeen
    : null,
};
const durationSec = (metric.durationMs / 1000).toFixed(1);
const cacheInfo = cache.total
  ? `${cache.hits}/${cache.total} (${Math.round((cache.hits / cache.total) * 100)}%)`
  : 'N/A';
const summaryMd = `### 🏁 CI Lane: \`${lane}\`\n\n| Metric | Value |\n| :--- | :--- |\n| **Status** | ✅ Passed |\n| **Duration** | ${durationSec}s |\n| **Nx Cache Hits** | ${cacheInfo} |\n| **Rollout Mode** | ${plan.shadow ? 'Shadow' : 'Affected'} |\n\nCI_METRIC ${JSON.stringify(metric)}\n`;
console.log(`CI_METRIC ${JSON.stringify(metric)}`);
appendFileSync(process.env.GITHUB_STEP_SUMMARY, `\n${summaryMd}\n`);
