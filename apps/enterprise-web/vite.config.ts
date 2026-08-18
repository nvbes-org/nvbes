import path from 'node:path';
import tailwindcss from '@tailwindcss/vite';
import react from '@vitejs/plugin-react';
import { visualizer } from 'rollup-plugin-visualizer';
import devtoolsJson from 'vite-plugin-devtools-json';
import { defineConfig, loadEnv, type Plugin, type PluginOption } from 'vite-plus';
import {
  browserIsolationHeaders,
  buildWebCsp,
  cspMetaFromHeader,
  integrityPolicyScripts,
  permissionsPolicy,
  productionTransportHeaders,
  uaClientHintsHeaders,
} from '../../libs/ts/web-runtime/src/csp';
import { observabilitySourceMapPlugins } from '../../tools/web-build/vite-observability-sourcemaps';
import { sriPlugin } from '../../tools/web-build/vite-sri';

function getCsp(mode: string, stripeJsUrl: string, stripeApiUrl: string): string {
  const isDev = mode === 'development';

  return buildWebCsp({
    mode,
    scriptSrc: stripeJsUrl ? [stripeJsUrl] : [],
    styleSrc: ['https://fonts.googleapis.com'],
    imgSrc: ['https:'],
    fontSrc: ['https://fonts.gstatic.com'],
    connectSrc: [
      ...(stripeApiUrl ? [stripeApiUrl] : []),
      ...(isDev ? ['http://localhost:8080'] : []),
    ],
    frameSrc: stripeJsUrl ? [stripeJsUrl] : [],
  });
}

function getMetaCsp(mode: string, stripeJsUrl: string, stripeApiUrl: string): string {
  return cspMetaFromHeader(getCsp(mode, stripeJsUrl, stripeApiUrl));
}

function pluginList(plugin: unknown): PluginOption[] {
  return Array.isArray(plugin) ? (plugin as PluginOption[]) : [plugin as PluginOption];
}

function cspPlugin(mode: string, stripeJsUrl: string, stripeApiUrl: string): Plugin {
  return {
    name: 'csp-injection-plugin',
    transformIndexHtml(html: string) {
      const cspString = getMetaCsp(mode, stripeJsUrl, stripeApiUrl);
      const metaTag = `<meta http-equiv="Content-Security-Policy" content="${cspString}" />`;
      return html.replace('<!-- %CSP_META% -->', metaTag);
    },
  };
}

export default defineConfig(({ command, mode }) => {
  const localEnv = loadEnv(mode, process.cwd(), '');
  const rootEnv = loadEnv(mode, path.resolve(process.cwd(), '../../'), '');
  const envSources = [process.env, localEnv, rootEnv];

  const stripeEnabled =
    process.env.VITE_STRIPE_ENABLED === 'true' ||
    localEnv.VITE_STRIPE_ENABLED === 'true' ||
    rootEnv.VITE_STRIPE_ENABLED === 'true' ||
    !!(
      process.env.VITE_STRIPE_PUBLISHABLE_KEY ||
      localEnv.VITE_STRIPE_PUBLISHABLE_KEY ||
      rootEnv.VITE_STRIPE_PUBLISHABLE_KEY
    );

  const stripeJsUrl =
    process.env.VITE_STRIPE_JS_URL ||
    localEnv.VITE_STRIPE_JS_URL ||
    rootEnv.VITE_STRIPE_JS_URL ||
    (stripeEnabled ? 'https://js.stripe.com' : '');

  const stripeApiUrl =
    process.env.VITE_STRIPE_API_URL ||
    localEnv.VITE_STRIPE_API_URL ||
    rootEnv.VITE_STRIPE_API_URL ||
    (stripeEnabled ? 'https://api.stripe.com' : '');

  const accountServiceBaseUrl =
    process.env.VITE_ACCOUNT_SERVICE_BASE_URL ||
    localEnv.VITE_ACCOUNT_SERVICE_BASE_URL ||
    rootEnv.VITE_ACCOUNT_SERVICE_BASE_URL ||
    'http://localhost:4000';

  const cspHeader = getCsp(mode, stripeJsUrl, stripeApiUrl);

  return {
    plugins: [
      ...pluginList(tailwindcss()),
      ...pluginList(react()),
      devtoolsJson(),
      cspPlugin(mode, stripeJsUrl, stripeApiUrl),
      ...(process.env.ANALYZE
        ? [
            visualizer({
              filename: 'dist/stats.html',
              gzipSize: true,
              brotliSize: true,
            }),
          ]
        : []),
      ...observabilitySourceMapPlugins({
        appName: 'enterprise-web',
        command,
        envSources,
      }),
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
          find: '@nvbes/identity-sdk-web',
          replacement: path.resolve(__dirname, '../../libs/ts/identity-sdk-web/src/index.ts'),
        },
        {
          find: /^@nvbes\/identity-sdk-web\/src\//,
          replacement: `${path.resolve(__dirname, '../../libs/ts/identity-sdk-web/src/')}/`,
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
          find: '@nvbes/web-ui',
          replacement: path.resolve(__dirname, '../../libs/ts/web-ui/src/index.ts'),
        },
      ],
    },
    server: {
      port: 5175,
      strictPort: true,
      host: 'localhost',
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
          target: accountServiceBaseUrl,
          changeOrigin: true,
          rewrite: (requestPath: string) => requestPath.replace(/^\/api\/v1/u, ''),
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
      port: 5175,
      strictPort: true,
      host: 'localhost',
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
