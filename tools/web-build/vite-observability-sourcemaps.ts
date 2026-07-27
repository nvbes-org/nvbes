import { spawnSync } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import faroUploader from '@grafana/faro-rollup-plugin';
import { sentryVitePlugin } from '@sentry/vite-plugin';
import type { PluginOption } from 'vite-plus';

type EnvSource = Record<string, string | undefined>;

type SourceMapPluginsOptions = {
  appName: string;
  envSources: EnvSource[];
};

export function observabilitySourceMapPlugins({
  appName,
  envSources,
}: SourceMapPluginsOptions): PluginOption[] {
  const release = releaseName(envSources);

  return [
    ...grafanaPlugins(appName, release, envSources),
    ...sentryPlugins(appName, release, envSources),
    ...posthogPlugins(appName, release, envSources),
  ];
}

function grafanaPlugins(
  appName: string,
  release: string | undefined,
  envSources: EnvSource[],
): PluginOption[] {
  const config = completeConfig(
    'Grafana',
    {
      endpoint: envValue('GRAFANA_FARO_SOURCEMAP_ENDPOINT', envSources),
      apiKey: envValue('GRAFANA_FARO_SOURCEMAP_API_KEY', envSources),
      appId: appEnvValue('GRAFANA_FARO_APP_ID', appName, envSources),
      stackId: envValue('GRAFANA_CLOUD_STACK_ID', envSources),
      bundleId: release,
    },
    appName,
  );
  if (!config) {
    return [];
  }

  return pluginList(
    faroUploader({
      appName,
      endpoint: config.endpoint,
      apiKey: config.apiKey,
      appId: config.appId,
      stackId: config.stackId,
      bundleId: config.bundleId,
      gitHash: config.bundleId,
      keepSourcemaps: true,
      recursive: true,
      skipUpload: true,
    }),
  );
}

function sentryPlugins(
  appName: string,
  release: string | undefined,
  envSources: EnvSource[],
): PluginOption[] {
  const config = completeConfig(
    'Sentry',
    {
      authToken: envValue('SENTRY_AUTH_TOKEN', envSources),
      org: envValue('SENTRY_ORG', envSources),
      project: appEnvValue('SENTRY_PROJECT', appName, envSources),
      release,
    },
    appName,
  );
  if (!config) {
    return [];
  }

  return pluginList(
    sentryVitePlugin({
      authToken: config.authToken,
      org: config.org,
      project: config.project,
      url: envValue('SENTRY_URL', envSources),
      telemetry: false,
      release: {
        name: config.release,
        inject: true,
        create: true,
        finalize: true,
        setCommits: { auto: true },
      },
      sourcemaps: {
        assets: ['./dist/**/*.js', './dist/**/*.js.map'],
      },
    }),
  );
}

function posthogPlugins(
  appName: string,
  release: string | undefined,
  envSources: EnvSource[],
): PluginOption[] {
  if (!envFlag('POSTHOG_SOURCEMAP_UPLOAD_ENABLED', envSources)) {
    return [];
  }

  const config = completeConfig(
    'PostHog',
    {
      personalApiKey:
        envValue('POSTHOG_CLI_API_KEY', envSources) ?? envValue('POSTHOG_API_KEY', envSources),
      projectId:
        envValue('POSTHOG_CLI_PROJECT_ID', envSources) ??
        envValue('POSTHOG_PROJECT_ID', envSources),
      release,
    },
    appName,
  );
  if (!config) {
    return [];
  }

  const host =
    envValue('POSTHOG_CLI_HOST', envSources) ??
    envValue('POSTHOG_HOST', envSources) ??
    'https://eu.posthog.com';

  return [
    {
      name: 'nvbes-posthog-source-map-upload',
      writeBundle: {
        sequential: true,
        handler(outputOptions) {
          const outputDirectory = path.resolve(outputOptions.dir ?? 'dist');
          const cliEnv = {
            POSTHOG_CLI_API_KEY: config.personalApiKey,
            POSTHOG_CLI_PROJECT_ID: config.projectId,
          };
          runPosthogCli(
            ['--host', host, 'sourcemap', 'inject', '--directory', outputDirectory],
            cliEnv,
          );
          runPosthogCli(
            [
              '--host',
              host,
              'sourcemap',
              'upload',
              '--directory',
              outputDirectory,
              '--release-name',
              appName,
              '--release-version',
              config.release,
            ],
            cliEnv,
          );
        },
      },
    } as PluginOption,
  ];
}

function completeConfig<T extends Record<string, string | undefined>>(
  provider: string,
  values: T,
  appName: string,
): { [K in keyof T]: string } | null {
  const configured = Object.values(values).filter(Boolean).length;
  if (configured === 0) {
    return null;
  }

  const missing = Object.entries(values)
    .filter(([, value]) => !value)
    .map(([key]) => key);
  if (missing.length > 0) {
    throw new Error(
      `${provider} source map upload for ${appName} is partially configured; missing ${missing.join(', ')}.`,
    );
  }

  return values as { [K in keyof T]: string };
}

function releaseName(envSources: EnvSource[]): string | undefined {
  return (
    envValue('NVBES_RELEASE', envSources) ??
    envValue('VITE_NVBES_BUILD_ID', envSources) ??
    envValue('GITHUB_SHA', envSources)
  );
}

function appEnvValue(prefix: string, appName: string, envSources: EnvSource[]): string | undefined {
  const suffix = appName.replaceAll('-', '_').toUpperCase();
  return envValue(`${prefix}_${suffix}`, envSources) ?? envValue(prefix, envSources);
}

function envValue(key: string, envSources: EnvSource[]): string | undefined {
  for (const source of envSources) {
    const value = source[key]?.trim();
    if (value) {
      return value;
    }
  }
  return undefined;
}

function envFlag(key: string, envSources: EnvSource[]): boolean {
  return envValue(key, envSources)?.toLowerCase() === 'true';
}

function pluginList(plugin: unknown): PluginOption[] {
  return Array.isArray(plugin) ? (plugin as PluginOption[]) : [plugin as PluginOption];
}

function runPosthogCli(args: string[], env: Record<string, string>): void {
  const binary = fileURLToPath(new URL('../../node_modules/.bin/posthog-cli', import.meta.url));
  const result = spawnSync(binary, args, {
    env: { ...process.env, ...env },
    stdio: 'inherit',
  });
  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0) {
    throw new Error(
      `PostHog source map command failed with exit code ${result.status ?? 'unknown'}.`,
    );
  }
}
