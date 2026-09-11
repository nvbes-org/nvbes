import { spawnSync } from 'node:child_process';
import { rmSync } from 'node:fs';
import path from 'node:path';
import { checkTypescriptReport, typescriptScope } from './typescript-quality-gate.mjs';
import { createMutationCheckpoint, executeMutationSteps } from './mutation-campaign-state.mjs';
import {
  mutationCheckout,
  mutationRunPaths,
  publishMutationReport,
} from './mutation-campaign-artifacts.mjs';

const name = process.env.NVBES_MUTATION_PACKAGE;
if (!/^[a-z][a-z0-9-]*$/u.test(name ?? '')) throw new Error('NVBES_MUTATION_PACKAGE is required');
typescriptScope(name);
const root = process.cwd();
const before = mutationCheckout(root);
const checkpoint = createMutationCheckpoint(root, name);
const directory = path.relative(root, path.dirname(checkpoint.file)).split(path.sep).join('/');
const { report } = mutationRunPaths(name, directory);
rmSync(`.temp/typescript/${name}/mutation.json`, { force: true });
console.log(`Mutation execution diagnostic: ${checkpoint.file}`);
process.exitCode = executeMutationSteps({
  context: {
    package: name,
    ...before,
    pid: process.pid,
    report,
  },
  steps: [
    {
      name: 'calibration',
      timeout: 30_000,
      args: ['--test', 'tools/test-summary/stryker-instrumentation.test.mjs'],
    },
    {
      name: 'mutation',
      timeout: 30 * 60 * 1000,
      args: [
        'node_modules/@stryker-mutator/core/bin/stryker.js',
        'run',
        'tools/test-summary/stryker.config.mjs',
      ],
    },
    {
      name: 'threshold-gate',
      timeout: 30_000,
      args: ['tools/test-summary/typescript-quality-gate.mjs', name, 'mutation', report],
    },
  ],
  run: ({ args, timeout }) =>
    spawnSync('node', args, {
      stdio: 'inherit',
      timeout,
      env: { ...process.env, NVBES_MUTATION_RUN_DIRECTORY: directory },
    }),
  persist: (state) => checkpoint.persist(state),
  finalize: () =>
    publishMutationReport({
      root,
      name,
      report,
      before,
      after: mutationCheckout(root),
      validate: (value) => checkTypescriptReport(name, 'mutation', value, root),
    }),
});
