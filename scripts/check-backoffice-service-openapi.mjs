#!/usr/bin/env node
import { execFileSync } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const ROOT_DIR = path.resolve(__dirname, '..');
const APP_OPENAPI = 'apps/backoffice-service/openapi.json';
const SDK_OPENAPI = 'libs/ts/backoffice-service-sdk-core/openapi.json';
const SDK_TYPES = 'libs/ts/backoffice-service-sdk-core/src/types.gen.ts';

function fail(message) {
  console.error(`Backoffice OpenAPI check failed: ${message}`);
  process.exit(1);
}

function read(pathname) {
  const absolute = path.join(ROOT_DIR, pathname);
  if (!existsSync(absolute)) {
    fail(`${pathname} is missing`);
  }
  return readFileSync(absolute, 'utf8');
}

function normalizeOpenApi(content) {
  return `${JSON.stringify(JSON.parse(content), null, 2)}\n`;
}

function assertEqual(pathname, actual, expected) {
  if (actual !== expected) {
    fail(`${pathname} is stale. Run pnpm generate:openapi.`);
  }
}

const exported = execFileSync(
  'cargo',
  ['run', '-p', 'nvbes-backoffice-service', '--', '--export-openapi'],
  {
    cwd: ROOT_DIR,
    encoding: 'utf8',
    maxBuffer: 20 * 1024 * 1024,
    env: process.env,
  },
);
const normalized = normalizeOpenApi(exported);

assertEqual(APP_OPENAPI, read(APP_OPENAPI), normalized);
assertEqual(SDK_OPENAPI, read(SDK_OPENAPI), normalized);

const tempDir = mkdtempSync(path.join(tmpdir(), 'nvbes-backoffice-service-openapi-'));
try {
  const tempSpec = path.join(tempDir, 'openapi.json');
  const tempTypes = path.join(tempDir, 'types.gen.ts');
  writeFileSync(tempSpec, normalized);
  execFileSync(
    'pnpm',
    [
      '--dir',
      'libs/ts/backoffice-service-sdk-core',
      'exec',
      'openapi-typescript',
      tempSpec,
      '-o',
      tempTypes,
    ],
    {
      cwd: ROOT_DIR,
      stdio: 'pipe',
      encoding: 'utf8',
      env: process.env,
    },
  );
  assertEqual(SDK_TYPES, read(SDK_TYPES), readFileSync(tempTypes, 'utf8'));
} finally {
  rmSync(tempDir, { force: true, recursive: true });
}

console.log('Backoffice OpenAPI and generated SDK types are up to date.');
