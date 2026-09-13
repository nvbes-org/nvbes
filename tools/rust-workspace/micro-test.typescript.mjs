import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
import path from 'node:path';

export function emailTestArguments() {
  // Resolve the same installed Vitest as vite-plus/test, without running the
  // vp launcher: its runtime provisioning may contact the npm registry.
  const require = createRequire(import.meta.url);
  const bundled = createRequire(require.resolve('vite-plus/package.json'));
  const manifest = bundled.resolve('vitest/package.json');
  const pkg = JSON.parse(readFileSync(manifest, 'utf8'));
  const bin = typeof pkg.bin === 'string' ? pkg.bin : pkg.bin.vitest;
  return [
    path.resolve(path.dirname(manifest), bin),
    'run',
    '--root',
    'libs/ts/email-ui',
    '--retry=0',
    '--maxWorkers=1',
  ];
}
