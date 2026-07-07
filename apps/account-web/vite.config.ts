import path from 'node:path';
import tailwindcss from '@tailwindcss/vite';
import react from '@vitejs/plugin-react';
import { visualizer } from 'rollup-plugin-visualizer';
import devtoolsJson from 'vite-plugin-devtools-json';
import { VitePWA } from 'vite-plugin-pwa';
import { defineConfig, loadEnv, type PluginOption } from 'vite-plus';
import {
  cspPlugin,
  getCsp,
  integrityPolicyStyles,
  originFromUrl,
  permissionsPolicy,
} from './identity.vite.csp';
import { identitySentryBuildSourcemap, identitySentryPlugins } from './identity.vite.sentry';
import { sriPlugin } from './identity.vite.sri';

function pluginList(plugin: unknown): PluginOption[] {
  return Array.isArray(plugin) ? (plugin as PluginOption[]) : [plugin as PluginOption];
}

export default defineConfig(({ mode }) => {
  const localEnv = loadEnv(mode, process.cwd(), '');
  const rootEnv = loadEnv(mode, path.resolve(process.cwd(), '../../'), '');
  const envSources = [process.env, localEnv, rootEnv];

  const accountServiceBaseUrl =
    process.env.VITE_ACCOUNT_SERVICE_BASE_URL ||
    localEnv.VITE_ACCOUNT_SERVICE_BASE_URL ||
    rootEnv.VITE_ACCOUNT_SERVICE_BASE_URL ||
    'http://localhost:4000';

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
  const faroUrl =
    process.env.VITE_FARO_URL ||
    localEnv.VITE_FARO_URL ||
    rootEnv.VITE_FARO_URL ||
    process.env.VITE_GRAFANA_FARO_URL ||
    localEnv.VITE_GRAFANA_FARO_URL ||
    rootEnv.VITE_GRAFANA_FARO_URL ||
    '';
  const faroConnectUrl = originFromUrl(faroUrl);
  const cspHeader = getCsp(mode, sentryConnectUrl, posthogConnectUrl, faroConnectUrl);

  return {
    plugins: [
      ...pluginList(tailwindcss()),
      ...pluginList(react()),
      devtoolsJson(),
      cspPlugin(mode, sentryConnectUrl, posthogConnectUrl, faroConnectUrl),
      sriPlugin(),
      ...pluginList(
        VitePWA({
          registerType: 'autoUpdate',
          injectRegister: false,
          strategies: 'injectManifest',
          srcDir: 'src',
          filename: 'sw.ts',
          injectManifest: {
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
      ...identitySentryPlugins(...envSources),
    ].filter(Boolean),
    build: {
      target: 'esnext',
      sourcemap: identitySentryBuildSourcemap(...envSources),
      minify: true,
      cssMinify: 'esbuild',
      manifest: true,
      modulePreload: { polyfill: false },
      rolldownOptions: {
        output: {
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
      port: 3001,
      strictPort: true,
      host: '0.0.0.0',
      headers: {
        'Content-Security-Policy': cspHeader,
        'Integrity-Policy-Report-Only': integrityPolicyStyles,
        'Expect-CT': 'max-age=86400, enforce',
        'X-Frame-Options': 'DENY',
        'X-Content-Type-Options': 'nosniff',
        'Referrer-Policy': 'strict-origin-when-cross-origin',
        'Permissions-Policy': permissionsPolicy,
        'X-XSS-Protection': '1; mode=block',
        'Cross-Origin-Opener-Policy': 'same-origin',
        'Cross-Origin-Resource-Policy': 'same-origin',
      },
      proxy: {
        '/api': {
          target: accountServiceBaseUrl,
          changeOrigin: true,
        },
        '/auth': {
          target: accountServiceBaseUrl,
          changeOrigin: true,
        },
        '/oauth': {
          target: accountServiceBaseUrl,
          changeOrigin: true,
        },
        '/csp-report': {
          target: accountServiceBaseUrl,
          changeOrigin: true,
        },
        '/workspaces': {
          target: accountServiceBaseUrl,
          changeOrigin: true,
        },
        '/legal': {
          target: accountServiceBaseUrl,
          changeOrigin: true,
        },
      },
    },
    preview: {
      port: 3001,
      strictPort: true,
      host: '0.0.0.0',
      headers: {
        'Content-Security-Policy': cspHeader,
        'Integrity-Policy': integrityPolicyStyles,
        'Integrity-Policy-Report-Only': integrityPolicyStyles,
        'Expect-CT': 'max-age=86400, enforce',
        'X-Frame-Options': 'DENY',
        'X-Content-Type-Options': 'nosniff',
        'Referrer-Policy': 'strict-origin-when-cross-origin',
        'Permissions-Policy': permissionsPolicy,
        'X-XSS-Protection': '1; mode=block',
        'Cross-Origin-Opener-Policy': 'same-origin',
        'Cross-Origin-Resource-Policy': 'same-origin',
      },
    },
  };
});
