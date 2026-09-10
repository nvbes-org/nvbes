import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { readFileSync, rmSync } from 'node:fs';
import path from 'node:path';
import { checkTypescriptReport, typescriptScope } from './typescript-quality-gate.mjs';

const name = process.env.NVBES_COVERAGE_PACKAGE;
assert(/^[a-z][a-z0-9-]*$/u.test(name ?? ''), 'NVBES_COVERAGE_PACKAGE is required');
typescriptScope(name);

const root = `libs/ts/${name}`;
const reports = path.resolve(`.temp/typescript/${name}/coverage`);
rmSync(reports, { force: true, recursive: true });

const run = spawnSync(
  'node',
  [
    'node_modules/vite-plus/bin/vp',
    'test',
    'run',
    '--root',
    root,
    '--retry=0',
    '--coverage',
    '--coverage.provider=v8',
    '--coverage.reporter=json-summary',
    `--coverage.reportsDirectory=${reports}`,
    '--coverage.include=src/**/*.{ts,tsx}',
    '--coverage.exclude=src/**/*.{test,spec,gen,d}.{ts,tsx}',
  ],
  { stdio: 'inherit', timeout: 10 * 60 * 1000 },
);

if (run.error || run.signal || run.status !== 0) {
  console.error('Coverage campaign failed or exceeded its 10-minute budget');
  process.exitCode = 1;
} else {
  try {
    const report = JSON.parse(readFileSync(path.join(reports, 'coverage-summary.json'), 'utf8'));
    console.log(JSON.stringify(checkTypescriptReport(name, 'coverage', report)));
  } catch (error) {
    console.error(`Coverage gate refused: ${error.message}`);
    process.exitCode = 1;
  }
}
