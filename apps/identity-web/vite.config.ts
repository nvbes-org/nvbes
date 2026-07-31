import path from 'node:path';
import tailwindcss from '@tailwindcss/vite';
import react from '@vitejs/plugin-react';
import { visualizer } from 'rollup-plugin-visualizer';
import devtoolsJson from 'vite-plugin-devtools-json';
import { VitePWA } from 'vite-plugin-pwa';
import { defineConfig, loadEnv, type PluginOption } from 'vite-plus';
import { observabilitySourceMapPlugins } from '../../tools/web-build/vite-observability-sourcemaps';
import { sriPlugin } from '../../tools/web-build/vite-sri';
import {
  browserIsolationHeaders,
  cspPlugin,
  getCsp,
  integrityPolicyScripts,
  originFromUrl,
  permissionsPolicy,
  productionTransportHeaders,
  uaClientHintsHeaders,
} from './identity.vite.csp';
import { resolveFaroUrl } from './identity.vite.observability';

const workspaceRoot = path.resolve(__dirname, '../..');

const reactRuntimeAliases = [
  {
    find: /^react$/,
    replacement: path.resolve(__dirname, '../../node_modules/react/index.js'),
  },
  {
    find: /^react\/jsx-runtime$/,
    replacement: path.resolve(__dirname, '../../node_modules/react/jsx-runtime.js'),
  },
  {
    find: /^react\/jsx-dev-runtime$/,
    replacement: path.resolve(__dirname, '../../node_modules/react/jsx-dev-runtime.js'),
  },
  {
    find: /^react-dom$/,
    replacement: path.resolve(__dirname, '../../node_modules/react-dom/index.js'),
  },
  {
    find: /^react-dom\/client$/,
    replacement: path.resolve(__dirname, '../../node_modules/react-dom/client.js'),
  },
];

function pluginList(plugin: unknown): PluginOption[] {
  return Array.isArray(plugin) ? (plugin as PluginOption[]) : [plugin as PluginOption];
}

export default defineConfig(({ mode }) => {
  const localEnv = loadEnv(mode, process.cwd(), '');
  const rootEnv = loadEnv(mode, workspaceRoot, '');
  const envSources = [process.env, localEnv, rootEnv];

  const identityServiceBaseUrl =
    process.env.VITE_IDENTITY_SERVICE_BASE_URL ||
    localEnv.VITE_IDENTITY_SERVICE_BASE_URL ||
    rootEnv.VITE_IDENTITY_SERVICE_BASE_URL ||
    'http://localhost:4000';
  const identityWebPort = Number(
    process.env.VITE_IDENTITY_WEB_PORT ||
      localEnv.VITE_IDENTITY_WEB_PORT ||
      rootEnv.VITE_IDENTITY_WEB_PORT ||
      3000,
  );
  if (!Number.isInteger(identityWebPort) || identityWebPort < 1024 || identityWebPort > 65_535) {
    throw new Error('VITE_IDENTITY_WEB_PORT must be an integer between 1024 and 65535');
  }

  const sentryDsn =
    process.env.VITE_SENTRY_DSN || localEnv.VITE_SENTRY_DSN || rootEnv.VITE_SENTRY_DSN || '';
  const sentryConnectUrl = originFromUrl(sentryDsn);
  const posthogKey =
    process.env.VITE_POSTHOG_KEY || localEnv.VITE_POSTHOG_KEY || rootEnv.VITE_POSTHOG_KEY || '';
  const posthogHost =
    process.env.VITE_POSTHOG_HOST ||
    localEnv.VITE_POSTHOG_HOST ||
    rootEnv.VITE_POSTHOG_HOST ||
    (posthogKey ? 'https://eu.i.posthog.com' : '');
  const posthogConnectUrl = originFromUrl(posthogHost);
  const faroUrl = resolveFaroUrl(envSources);
  const faroConnectUrl = originFromUrl(faroUrl);
  const isLocalEnvironment =
    (process.env.NVBES_ENV || localEnv.NVBES_ENV || rootEnv.NVBES_ENV) === 'development';
  const cspHeader = getCsp(
    mode,
    sentryConnectUrl,
    posthogConnectUrl,
    faroConnectUrl,
    isLocalEnvironment,
  );

  return {
    envDir: workspaceRoot,
    plugins: [
      ...pluginList(tailwindcss()),
      ...pluginList(react()),
      devtoolsJson(),
      cspPlugin(mode, sentryConnectUrl, posthogConnectUrl, faroConnectUrl, isLocalEnvironment),
      ...pluginList(
        VitePWA({
          registerType: 'autoUpdate',
          injectRegister: false,
          strategies: 'injectManifest',
          srcDir: 'src',
          filename: 'sw.ts',
          injectManifest: {
            globIgnores: [
              '**/analytics-posthog-*.js',
              '**/analytics-sentry-*.js',
              '**/password-strength-*.js',
              '**/stats.html',
            ],
            rollupFormat: 'iife',
            sourcemap: false,
          },
          includeAssets: ['icon.svg', 'icon-180.png', 'icon-192.png', 'icon-512.png'],
          manifest: false,
        }),
      ),
      ...(process.env.ANALYZE
        ? [
            visualizer({
              filename: 'dist/stats.html',
              gzipSize: true,
              brotliSize: true,
            }),
          ]
        : []),
      ...observabilitySourceMapPlugins({ appName: 'identity-web', envSources }),
      sriPlugin(),
    ].filter(Boolean),
    build: {
      target: 'esnext',
      sourcemap: 'hidden',
      minify: true,
      cssMinify: 'esbuild',
      manifest: true,
      modulePreload: { polyfill: false },
      rolldownOptions: {
        output: {
          codeSplitting: {
            groups: [
              {
                name: 'react-runtime',
                test: /node_modules[\\/](?:react|react-dom|scheduler)[\\/]/,
                priority: 40,
              },
              {
                name: 'tanstack-runtime',
                test: /node_modules[\\/]@tanstack[\\/]/,
                priority: 30,
              },
              {
                name: 'analytics-sentry',
                test: /node_modules[\\/]@sentry[\\/]/,
                priority: 25,
              },
              {
                name: 'analytics-posthog',
                test: /node_modules[\\/]posthog-js[\\/]/,
                priority: 25,
              },
              {
                name: 'password-strength',
                test: /node_modules[\\/]@zxcvbn-ts[\\/]/,
                minSize: 20_000,
                maxSize: 450_000,
                priority: 20,
              },
            ],
          },
          minify: {
            compress: {
              dropConsole: mode === 'production',
              dropDebugger: mode === 'production',
            },
          },
        },
        treeshake: true,
      },
    },
    resolve: {
      dedupe: ['react', 'react-dom', 'react/jsx-runtime', 'react/jsx-dev-runtime'],
      alias: [
        ...reactRuntimeAliases,
        {
          find: '@',
          replacement: path.resolve(__dirname, './src'),
        },
        {
          find: '@nvbes/http-client',
          replacement: path.resolve(__dirname, '../../libs/ts/http-client/src/index.ts'),
        },
        {
          find: '@nvbes/identity-client',
          replacement: path.resolve(__dirname, '../../libs/ts/identity-client/src/index.ts'),
        },
        {
          find: '@nvbes/web-runtime/analytics',
          replacement: path.resolve(__dirname, '../../libs/ts/web-runtime/src/analytics.ts'),
        },
        {
          find: '@nvbes/web-runtime',
          replacement: path.resolve(__dirname, '../../libs/ts/web-runtime/src/index.ts'),
        },
        {
          find: '@nvbes/identity-sdk-web',
          replacement: path.resolve(__dirname, '../../libs/ts/identity-sdk-web/src/index.ts'),
        },
        {
          find: '@nvbes/web-ui',
          replacement: path.resolve(__dirname, '../../libs/ts/web-ui/src/index.ts'),
        },
        {
          find: /^@nvbes\/identity-sdk-web\/src\//,
          replacement: `${path.resolve(__dirname, '../../libs/ts/identity-sdk-web/src/')}/`,
        },
      ],
    },
    server: {
      port: identityWebPort,
      strictPort: true,
      host: '0.0.0.0',
      headers: {
        ...uaClientHintsHeaders,
        ...browserIsolationHeaders,
        ...productionTransportHeaders(mode),
        'Content-Security-Policy': cspHeader,
        'Integrity-Policy-Report-Only': integrityPolicyScripts,
        'Expect-CT': 'max-age=86400, enforce',
        'X-Frame-Options': 'DENY',
        'Referrer-Policy': 'strict-origin-when-cross-origin',
        'Permissions-Policy': permissionsPolicy,
        'X-XSS-Protection': '1; mode=block',
      },
      proxy: {
        '/api': {
          target: identityServiceBaseUrl,
          changeOrigin: true,
        },
        '/auth': {
          target: identityServiceBaseUrl,
          changeOrigin: true,
        },
        '/oauth': {
          target: identityServiceBaseUrl,
          changeOrigin: true,
        },
        '/csp-report': {
          target: identityServiceBaseUrl,
          changeOrigin: true,
        },
      },
    },
    preview: {
      port: identityWebPort,
      strictPort: true,
      host: '0.0.0.0',
      headers: {
        ...uaClientHintsHeaders,
        ...browserIsolationHeaders,
        ...productionTransportHeaders(mode),
        'Content-Security-Policy': cspHeader,
        'Integrity-Policy-Report-Only': integrityPolicyScripts,
        'Expect-CT': 'max-age=86400, enforce',
        'X-Frame-Options': 'DENY',
        'Referrer-Policy': 'strict-origin-when-cross-origin',
        'Permissions-Policy': permissionsPolicy,
        'X-XSS-Protection': '1; mode=block',
      },
    },
  };
});
