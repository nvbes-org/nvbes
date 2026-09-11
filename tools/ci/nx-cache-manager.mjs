#!/usr/bin/env node
import { execFileSync } from 'node:child_process';
import { appendFileSync, readFileSync, writeFileSync } from 'node:fs';
import { createPlan, lanes } from './scope-plan.mjs';
import { loadTerraformSources } from './scope-terraform.mjs';
import { validateCommittedReport, validateRolloutPolicy } from './rollout-policy.mjs';
import { scopeMetric } from './rollout-metrics.mjs';

const preflight = JSON.parse(readFileSync('.nx/ci/preflight.json', 'utf8'));
const git = (...args) =>
  execFileSync('git', args, {
    encoding: 'utf8',
    maxBuffer: 20 * 1024 * 1024,
    stdio: ['ignore', 'pipe', 'ignore'],
  });
const nx = (...args) =>
  JSON.parse(
    execFileSync('pnpm', ['exec', 'nx', ...args], {
      encoding: 'utf8',
      maxBuffer: 20 * 1024 * 1024,
      env: { ...process.env, NX_NO_CLOUD: 'true' },
    }),
  );
let context;
if (preflight.graphRequired) {
  // A broken graph cannot enumerate validation targets: fail closed.
  const nodes = nx('graph', '--print').graph.nodes;
  for (const { data } of Object.values(nodes)) {
    const database = data.targets?.['test:database'];
    if (database && database.cache !== false)
      throw new Error(`${data.root}: database tests must not be cacheable`);
  }
  let affected = Object.keys(nodes);
  let graphFallback = false;
  try {
    if (!preflight.fallback)
      affected = nx(
        'show',
        'projects',
        '--affected',
        `--base=${preflight.base}`,
        `--head=${preflight.head}`,
        '--json',
      );
  } catch {
    graphFallback = true;
  }
  const revisions = [...new Set([preflight.base, preflight.head])];
  const files = [
    ...new Set(
      revisions.flatMap((revision) =>
        git('ls-tree', '-r', '--name-only', '-z', revision).split('\0').filter(Boolean),
      ),
    ),
  ];
  const readVersions = (path) =>
    revisions.flatMap((revision) => {
      try {
        return [git('show', `${revision}:${path}`)];
      } catch {
        return [];
      }
    });
  const read = (path) => readVersions(path).join('\n');
  let globalConfigurationChanged = false;
  if (preflight.paths.includes('package.json')) {
    try {
      const manifests = readVersions('package.json').map(JSON.parse);
      const execution = (manifest) =>
        JSON.stringify([manifest.scripts, manifest.engines, manifest.packageManager]);
      globalConfigurationChanged =
        manifests.length !== 2 || execution(manifests[0]) !== execution(manifests[1]);
    } catch {
      globalConfigurationChanged = true;
    }
  }
  const terraformSources = preflight.paths.some((path) => path.startsWith('infrastructure/'))
    ? await loadTerraformSources(files, readVersions)
    : undefined;
  context = { nodes, affected, terraformSources, read, graphFallback, globalConfigurationChanged };
}
const policy = validateRolloutPolicy(
  JSON.parse(readFileSync('tools/ci/rollout-policy.json', 'utf8')),
);
const rolloutMode = process.env.NVBES_CI_ROLLOUT_MODE || 'affected';
if (rolloutMode === 'affected' && policy.mode === 'affected' && policy.activationEvidence) {
  const report = JSON.parse(
    git('show', `${policy.activationEvidence.reportCommit}:docs/ci/affected-rollout-evidence.json`),
  );
  validateCommittedReport(policy, report);
}
const plan = createPlan(preflight, { ...context, rolloutMode });
console.log(`CI_SCOPE ${JSON.stringify(scopeMetric(plan, preflight.startedAt))}`);
writeFileSync('.nx/ci/plan.json', JSON.stringify(plan));
const output = [
  `plan=${JSON.stringify(plan)}`,
  `base-sha=${plan.base}`,
  `head-sha=${plan.head}`,
  `docs-only=${plan.docsOnly}`,
  `ci-only=${plan.ciOnly}`,
  `fallback-full=${plan.fallbackFull}`,
  `rust-mode=${plan.rustMode}`,
  ...lanes.map((lane) => `${lane}-required=${plan.candidate[lane]}`),
];
appendFileSync(process.env.GITHUB_OUTPUT, `${output.join('\n')}\n`);
appendFileSync(
  process.env.GITHUB_STEP_SUMMARY,
  `## CI scope\n\nBase: \`${plan.base}\` → head: \`${plan.head}\`\n\n${plan.reason}\n\nFallback full: **${plan.fallbackFull}**\n\n| Lane | Required |\n|---|---|\n${lanes.map((lane) => `| ${lane} | ${plan.candidate[lane]} |`).join('\n')}\n\nPlanning: ${Date.now() - preflight.startedAt} ms. Rust: **${plan.rustMode}**, scoped candidate recorded by the Rust lane.\n`,
);
