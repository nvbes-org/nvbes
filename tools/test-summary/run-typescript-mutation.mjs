import { execFileSync, spawnSync } from 'node:child_process';
import { rmSync } from 'node:fs';
import { typescriptScope } from './typescript-quality-gate.mjs';
import { createMutationCheckpoint, executeMutationSteps } from './mutation-campaign-state.mjs';

const name = process.env.NVBES_MUTATION_PACKAGE;
if (!/^[a-z][a-z0-9-]*$/u.test(name ?? '')) throw new Error('NVBES_MUTATION_PACKAGE is required');
typescriptScope(name);
const report = `.temp/typescript/${name}/mutation.json`;
rmSync(report, { force: true });
const checkpoint = createMutationCheckpoint(process.cwd(), name);
console.log(`Mutation execution diagnostic: ${checkpoint.file}`);
process.exitCode = executeMutationSteps({
  context: {
    package: name,
    sha: execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim(),
    trackedChanges:
      execFileSync('git', ['status', '--porcelain', '--untracked-files=no'], {
        encoding: 'utf8',
      }).trim() !== '',
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
  run: ({ args, timeout }) => spawnSync('node', args, { stdio: 'inherit', timeout }),
  persist: checkpoint.persist,
});
