#!/usr/bin/env node
import { spawnSync } from 'node:child_process';

const allowedLicenses = new Set([
  '(MIT OR CC0-1.0)',
  '(MPL-2.0 OR Apache-2.0)',
  '(Apache-2.0 AND MIT)',
  'Apache-2.0',
  'BSD-2-Clause',
  'BSD-3-Clause',
  'BlueOak-1.0.0',
  'CC0-1.0',
  'ISC',
  'MIT',
  'Unlicense',
]);

const result = spawnSync('pnpm', ['--silent', 'licenses', 'list', '--prod', '--json'], {
  encoding: 'utf8',
  maxBuffer: 32 * 1024 * 1024,
});

if (result.status !== 0) {
  process.stderr.write(result.stderr || result.stdout);
  process.exit(result.status ?? 1);
}

let report;
try {
  report = JSON.parse(result.stdout);
} catch {
  console.error('JavaScript license check failed: pnpm returned invalid JSON');
  process.exit(1);
}

const observed = Object.keys(report).sort();
const denied = observed.filter((license) => !allowedLicenses.has(license));
if (denied.length > 0) {
  console.error(`JavaScript license check failed: unreviewed licenses: ${denied.join(', ')}`);
  process.exit(1);
}

console.log(`JavaScript licenses: ok (${observed.length} expressions reviewed)`);
