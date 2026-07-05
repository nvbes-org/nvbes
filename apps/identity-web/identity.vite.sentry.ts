import { createRequire } from 'node:module';
import type { PluginOption } from 'vite-plus';

type EnvSource = Record<string, string | undefined>;

type SentryVitePluginOptions = {
  authToken?: string;
  org?: string;
  project?: string;
  url?: string;
  telemetry?: boolean;
  release?: {
    name?: string;
    inject?: boolean;
    create?: boolean;
    finalize?: boolean;
    setCommits?: { auto: true };
  };
  sourcemaps?: {
    assets?: string[];
    filesToDeleteAfterUpload?: string[];
  };
};

type SentryVitePluginModule = {
  sentryCliBinaryExists: () => boolean;
  sentryVitePlugin: (options: SentryVitePluginOptions) => PluginOption | PluginOption[];
};

const require = createRequire(import.meta.url);

export function identitySentryBuildSourcemap(...envSources: EnvSource[]): boolean | 'hidden' {
  return isSentrySourceMapUploadConfigured(...envSources) ? 'hidden' : true;
}

export function identitySentryPlugins(...envSources: EnvSource[]): PluginOption[] {
  if (!hasSentrySourceMapUploadCredentials(...envSources)) {
    return [];
  }

  const sentryPlugin = loadSentryVitePlugin();
  if (!sentryPlugin?.sentryCliBinaryExists()) {
    return [];
  }

  const options: SentryVitePluginOptions = {
    authToken: envValue('SENTRY_AUTH_TOKEN', envSources),
    org: envValue('SENTRY_ORG', envSources),
    project: envValue('SENTRY_PROJECT', envSources),
    url: envValue('SENTRY_URL', envSources),
    telemetry: false,
    release: {
      name: releaseName(envSources),
      inject: true,
      create: true,
      finalize: true,
      setCommits: { auto: true },
    },
    sourcemaps: {
      assets: [
        './dist/assets/**/*.js',
        './dist/assets/**/*.js.map',
        './dist/sw.js',
        './dist/sw.js.map',
      ],
      filesToDeleteAfterUpload: ['./dist/assets/**/*.map', './dist/*.map'],
    },
  };

  return pluginList(sentryPlugin.sentryVitePlugin(options));
}

function isSentrySourceMapUploadConfigured(...envSources: EnvSource[]): boolean {
  if (!hasSentrySourceMapUploadCredentials(...envSources)) {
    return false;
  }

  const sentryPlugin = loadSentryVitePlugin();
  return Boolean(sentryPlugin?.sentryCliBinaryExists());
}

function hasSentrySourceMapUploadCredentials(...envSources: EnvSource[]): boolean {
  return Boolean(
    envValue('SENTRY_AUTH_TOKEN', envSources) &&
    envValue('SENTRY_ORG', envSources) &&
    envValue('SENTRY_PROJECT', envSources),
  );
}

function loadSentryVitePlugin(): SentryVitePluginModule | null {
  try {
    return require('@sentry/vite-plugin') as SentryVitePluginModule;
  } catch (error) {
    if (isModuleNotFoundError(error)) {
      console.warn('Sentry source map upload disabled: @sentry/vite-plugin is not installed.');
      return null;
    }
    throw error;
  }
}

function isModuleNotFoundError(error: unknown): boolean {
  return (
    typeof error === 'object' &&
    error !== null &&
    'code' in error &&
    error.code === 'MODULE_NOT_FOUND'
  );
}

function pluginList(plugin: PluginOption | PluginOption[]): PluginOption[] {
  return Array.isArray(plugin) ? plugin : [plugin];
}

function releaseName(envSources: EnvSource[]): string | undefined {
  return (
    envValue('SENTRY_RELEASE', envSources) ??
    envValue('VITE_SENTRY_RELEASE', envSources) ??
    envValue('NVBES_RELEASE', envSources) ??
    envValue('VITE_NVBES_BUILD_ID', envSources)
  );
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
