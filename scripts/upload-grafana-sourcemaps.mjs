import { spawnSync } from 'node:child_process';
import { rmSync } from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';

const appName = process.argv[2];
const outputDirectory = path.resolve(process.argv[3] ?? 'dist');

if (!appName) {
  throw new Error('Usage: upload-grafana-sourcemaps.mjs <app-name> [output-directory]');
}

const config = completeConfig({
  endpoint: envValue('GRAFANA_FARO_SOURCEMAP_ENDPOINT'),
  apiKey: envValue('GRAFANA_FARO_SOURCEMAP_API_KEY'),
  appId: appEnvValue('GRAFANA_FARO_APP_ID', appName),
  stackId: envValue('GRAFANA_CLOUD_STACK_ID'),
  bundleId: envValue('NVBES_RELEASE') ?? envValue('VITE_NVBES_BUILD_ID') ?? envValue('GITHUB_SHA'),
});

if (config) {
  const appNameSuffix = appName.replaceAll(/[^a-zA-Z0-9]/g, '_').toUpperCase();
  rmSync(path.resolve(process.cwd(), `.env.${appNameSuffix}`), { force: true });

  const binary = fileURLToPath(new URL('../node_modules/.bin/faro-cli', import.meta.url));
  const result = spawnSync(
    binary,
    [
      'upload',
      '--endpoint',
      config.endpoint,
      '--app-id',
      config.appId,
      '--api-key',
      config.apiKey,
      '--stack-id',
      config.stackId,
      '--bundle-id',
      config.bundleId,
      '--app-name',
      appName,
      '--output-path',
      outputDirectory,
      '--keep-sourcemaps',
      '--gzip-contents',
      '--recursive',
    ],
    { stdio: 'inherit' },
  );

  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0) {
    throw new Error(
      `Grafana source map upload failed with exit code ${result.status ?? 'unknown'}.`,
    );
  }
}

function completeConfig(values) {
  const configured = Object.values(values).filter(Boolean).length;
  if (configured === 0) {
    return null;
  }

  const missing = Object.entries(values)
    .filter(([, value]) => !value)
    .map(([key]) => key);
  if (missing.length > 0) {
    throw new Error(
      `Grafana source map upload for ${appName} is partially configured; missing ${missing.join(', ')}.`,
    );
  }

  return values;
}

function appEnvValue(prefix, applicationName) {
  const suffix = applicationName.replaceAll('-', '_').toUpperCase();
  return envValue(`${prefix}_${suffix}`) ?? envValue(prefix);
}

function envValue(key) {
  const value = process.env[key]?.trim();
  return value || undefined;
}
