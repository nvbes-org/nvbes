import { readFileSync } from 'node:fs';

const name = process.env.NVBES_MUTATION_PACKAGE;
if (!/^[a-z][a-z0-9-]*$/u.test(name ?? '')) throw new Error('NVBES_MUTATION_PACKAGE is required');
const root = `libs/ts/${name}`;
const pkg = JSON.parse(readFileSync(`${root}/package.json`, 'utf8'));
const scope = JSON.parse(readFileSync('docs/testing/v1/manifest.json', 'utf8'));
if (Object.hasOwn(scope.typescriptExclusions, pkg.name))
  throw new Error('Package has no applicable runtime mutation metric');

export default {
  mutate: [`${root}/src/**/*.{ts,tsx}`, `!${root}/src/**/*.{test,spec,gen,d}.{ts,tsx}`],
  testRunner: 'command',
  commandRunner: {
    command: `node node_modules/vite-plus/bin/vp test run --root ${root} --retry=0`,
  },
  coverageAnalysis: 'off',
  concurrency: 1,
  timeoutMS: 10000,
  timeoutFactor: 1,
  dryRunTimeoutMinutes: 2,
  reporters: ['clear-text', 'json'],
  jsonReporter: { fileName: `.temp/typescript/${name}/mutation.json` },
  tempDirName: `.temp/stryker-${name}`,
  ignorePatterns: [
    '/target',
    '/archive',
    '/.temp',
    '/.trunk',
    '/.nx',
    '/.codex',
    '/.agents',
    '/.git',
    '/.github',
    '/.pnpm-store',
    '/apps',
    '/infrastructure',
    '/vendor',
    '/deploy',
    '/fuzz',
    '/reports',
    '/libs/rust',
  ],
  incremental: false,
  // Stryker counts timeouts as detected. nvbes evaluates the report separately
  // and counts them as missed, using check-typescript-mutation.mjs.
  thresholds: { high: 90, low: 90, break: 0 },
};
