import { spawnSync } from 'node:child_process';
import { rmSync } from 'node:fs';
import { typescriptScope } from './typescript-quality-gate.mjs';

const name = process.env.NVBES_MUTATION_PACKAGE;
if (!/^[a-z][a-z0-9-]*$/u.test(name ?? '')) throw new Error('NVBES_MUTATION_PACKAGE is required');
typescriptScope(name);
const report = `.temp/typescript/${name}/mutation.json`;
rmSync(report, { force: true });
const calibration = spawnSync(
  'node',
  ['--test', 'tools/test-summary/stryker-instrumentation.test.mjs'],
  { stdio: 'inherit', timeout: 30_000 },
);
if (calibration.error || calibration.signal || calibration.status !== 0) {
  console.error('Mutation instrumentation calibration failed');
  process.exit(1);
}
const run = spawnSync(
  'node',
  [
    'node_modules/@stryker-mutator/core/bin/stryker.js',
    'run',
    'tools/test-summary/stryker.config.mjs',
  ],
  { stdio: 'inherit', timeout: 30 * 60 * 1000 },
);
if (run.error || run.signal || run.status !== 0) {
  console.error('Mutation campaign failed or exceeded its 30-minute budget');
  process.exitCode = 1;
} else {
  const gate = spawnSync(
    'node',
    ['tools/test-summary/typescript-quality-gate.mjs', name, 'mutation', report],
    {
      stdio: 'inherit',
    },
  );
  process.exitCode = gate.status ?? 1;
}
