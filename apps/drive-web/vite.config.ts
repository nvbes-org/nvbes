import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { sentryVitePlugin } from '@sentry/vite-plugin';
import tailwindcss from '@tailwindcss/vite';
import react from '@vitejs/plugin-react';
import { visualizer } from 'rollup-plugin-visualizer';
import devtoolsJson from 'vite-plugin-devtools-json';
import { defineConfig, loadEnv, type Plugin, type PluginOption } from 'vite-plus';
import { VitePWA } from 'vite-plugin-pwa';

const permissionsPolicy =
  'accelerometer=(), camera=(), geolocation=(), gyroscope=(), magnetometer=(), microphone=(), payment=(), usb=()';

function getSentryHost(dsn?: string): string {
  if (!dsn) return '';
  try {
    const url = new URL(dsn);
    return `${url.protocol}//${url.host}`;
  } catch {
    return '';
  }
}

function getCsp(
  mode: string,
  sentryHost: string,
  stripeJsUrl: string,
  stripeApiUrl: string,
): string {
  const isDev = mode === 'development';
  const sentryScript = sentryHost ? ` ${sentryHost}` : '';
  const sentryConnect = sentryHost ? ` ${sentryHost}` : '';

  const stripeScript = stripeJsUrl ? ` ${stripeJsUrl}` : '';
  const stripeFrame = stripeJsUrl ? ` ${stripeJsUrl}` : '';
  const stripeConnect = stripeApiUrl ? ` ${stripeApiUrl}` : '';

  if (isDev) {
    return `default-src 'self'; script-src 'self' 'unsafe-inline' 'unsafe-eval' ${sentryScript} ${stripeScript}; worker-src 'self' blob:; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; img-src 'self' data: blob: https:; font-src 'self' data: https://fonts.gstatic.com; connect-src 'self' ws://localhost:* ws://127.0.0.1:* http://localhost:* http://127.0.0.1:* ${sentryConnect} ${stripeConnect} http://localhost:8080 http://127.0.0.1:8080; frame-src 'self' ${stripeFrame}; object-src 'none'; base-uri 'self'; form-action 'self'; frame-ancestors 'none'; report-uri /csp-report; upgrade-insecure-requests;`;
  }
  return `default-src 'self'; script-src 'self' ${sentryScript} ${stripeScript}; worker-src 'self' blob:; style-src 'self' https://fonts.googleapis.com; img-src 'self' data: blob: https:; font-src 'self' data: https://fonts.gstatic.com; connect-src 'self' ${sentryConnect} ${stripeConnect}; frame-src 'self' ${stripeFrame}; object-src 'none'; base-uri 'self'; form-action 'self'; frame-ancestors 'none'; report-uri /csp-report; upgrade-insecure-requests;`;
}

function getMetaCsp(
  mode: string,
  sentryHost: string,
  stripeJsUrl: string,
  stripeApiUrl: string,
): string {
  return getCsp(mode, sentryHost, stripeJsUrl, stripeApiUrl).replace(
    "; frame-ancestors 'none'",
    '',
  );
}

function pluginList(plugin: unknown): PluginOption[] {
  return Array.isArray(plugin) ? (plugin as PluginOption[]) : [plugin as PluginOption];
}

function cspPlugin(
  mode: string,
  sentryHost: string,
  stripeJsUrl: string,
  stripeApiUrl: string,
): Plugin {
  return {
    name: 'csp-injection-plugin',
    transformIndexHtml(html: string) {
      const cspString = getMetaCsp(mode, sentryHost, stripeJsUrl, stripeApiUrl);
      const metaTag = `<meta http-equiv="Content-Security-Policy" content="${cspString}" />`;
      return html.replace('<!-- %CSP_META% -->', metaTag);
    },
  };
}

function sriPlugin(): Plugin {
  let outDir: string;
  const hashes = new Map<string, string>();

  return {
    name: 'vite-plugin-sri',
    enforce: 'post',
    configResolved(config) {
      outDir = config.build.outDir;
    },
    generateBundle(_opts, bundle) {
      for (const [name, info] of Object.entries(bundle)) {
        const source = info.type === 'chunk' ? info.code : info.source;
        if (!source) continue;
        const input = typeof source === 'string' ? source : Buffer.from(source).toString('utf-8');
        const hash = crypto.createHash('sha384').update(input, 'utf-8').digest('base64');
        hashes.set(name, `sha384-${hash}`);
      }
    },
    closeBundle() {
      const htmlPath = path.resolve(outDir, 'index.html');
      if (!fs.existsSync(htmlPath)) return;
      let html = fs.readFileSync(htmlPath, 'utf-8');

      for (const [name, integrity] of hashes) {
        const escaped = name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
        html = html.replace(
          new RegExp(`(<script[^>]*src="[^"]*${escaped}"[^>]*)(>)`, 'g'),
          (match) => {
            if (match.includes('integrity=')) return match;
            return match.includes('crossorigin')
              ? match.replace('>', ` integrity="${integrity}">`)
              : match.replace('>', ` integrity="${integrity}" crossorigin="anonymous">`);
          },
        );
        html = html.replace(
          new RegExp(`(<link[^>]*href="[^"]*${escaped}"[^>]*)(>)`, 'g'),
          (match) => {
            if (match.includes('integrity=')) return match;
            return match.includes('crossorigin')
              ? match.replace('>', ` integrity="${integrity}">`)
              : match.replace('>', ` integrity="${integrity}" crossorigin="anonymous">`);
          },
        );
      }

      fs.writeFileSync(htmlPath, html);
    },
  };
}

export default defineConfig(({ mode }) => {
  const localEnv = loadEnv(mode, process.cwd(), '');
  const rootEnv = loadEnv(mode, path.resolve(process.cwd(), '../../'), '');

  const sentryDsn =
    process.env.VITE_SENTRY_DSN ||
    localEnv.VITE_SENTRY_DSN ||
    rootEnv.VITE_SENTRY_DSN ||
    process.env.NVBES_SENTRY_DSN ||
    localEnv.NVBES_SENTRY_DSN ||
    rootEnv.NVBES_SENTRY_DSN ||
    '';

  const sentryHost =
    process.env.VITE_SENTRY_INGEST_URL ||
    localEnv.VITE_SENTRY_INGEST_URL ||
    rootEnv.VITE_SENTRY_INGEST_URL ||
    getSentryHost(sentryDsn);

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

  const cspHeader = getCsp(mode, sentryHost, stripeJsUrl, stripeApiUrl);
  const sentryOrg = process.env.SENTRY_ORG || localEnv.SENTRY_ORG || rootEnv.SENTRY_ORG || 'nvbes';
  const sentryAuthToken =
    process.env.SENTRY_AUTH_TOKEN ||
    localEnv.SENTRY_AUTH_TOKEN ||
    rootEnv.SENTRY_AUTH_TOKEN ||
    '';
  const sentryProject =
    process.env.SENTRY_PROJECT_DRIVE_WEB ||
    localEnv.SENTRY_PROJECT_DRIVE_WEB ||
    rootEnv.SENTRY_PROJECT_DRIVE_WEB ||
    process.env.SENTRY_PROJECT ||
    localEnv.SENTRY_PROJECT ||
    rootEnv.SENTRY_PROJECT ||
    'drive-web';

  return {
    plugins: [
      ...pluginList(react()),
      ...pluginList(tailwindcss()),
      devtoolsJson(),
      cspPlugin(mode, sentryHost, stripeJsUrl, stripeApiUrl),
      sriPlugin(),
      ...pluginList(
        VitePWA({
          registerType: 'autoUpdate',
          strategies: 'injectManifest',
          srcDir: 'src',
          filename: 'sw.ts',
          injectManifest: {
            sourcemap: false,
          },
          includeAssets: ['icon.svg', 'offline.html'],
          manifest: {
            name: 'nvbes Drive',
            short_name: 'Drive',
            description: 'Stockage et partage de fichiers securise',
            theme_color: '#0a0a0a',
            background_color: '#0a0a0a',
            display: 'standalone',
            orientation: 'any',
            start_url: '/',
            icons: [
              {
                src: 'icon.svg',
                sizes: '512x512',
                type: 'image/svg+xml',
                purpose: 'any maskable',
              },
            ],
            shortcuts: [
              {
                name: 'Nouveau document',
                short_name: 'Nouveau',
                description: 'Creer un nouveau document',
                url: '/new',
                icons: [{ src: 'icon.svg', sizes: '96x96' }],
              },
              {
                name: 'Scanner',
                short_name: 'Scan',
                description: 'Numeriser un document',
                url: '/scan',
                icons: [{ src: 'icon.svg', sizes: '96x96' }],
              },
            ],
            share_target: {
              action: '/share',
              method: 'POST',
              enctype: 'multipart/form-data',
              params: {
                title: 'title',
                text: 'text',
                url: 'url',
                files: [{ name: 'files', accept: ['*/*'] }],
              },
            },
            file_handlers: [
              {
                action: '/open',
                accept: { 'application/octet-stream': ['.nvbes'] },
              },
            ],
          },
        }),
      ),
      ...(sentryAuthToken
        ? pluginList(
            sentryVitePlugin({
              authToken: sentryAuthToken,
              org: sentryOrg,
              project: sentryProject,
              sourcemaps: {
                filesToDeleteAfterUpload: ['./dist/**/*.map'],
              },
            }),
          )
        : []),
      ...(process.env.ANALYZE
        ? [
            visualizer({
              filename: 'dist/stats.html',
              gzipSize: true,
              brotliSize: true,
            }),
          ]
        : []),
    ].filter(Boolean),
    build: {
      target: 'esnext',
      sourcemap: true,
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
          find: '@nvbes/identity-sdk',
          replacement: path.resolve(__dirname, '../../libs/ts/identity-sdk/src/index.ts'),
        },
        {
          find: '@nvbes/web-runtime',
          replacement: path.resolve(__dirname, '../../libs/ts/web-runtime/src/index.ts'),
        },
      ],
    },
    server: {
      host: '0.0.0.0',
      port: 5173,
      headers: {
        'Content-Security-Policy': cspHeader,
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
    preview: {
      host: '0.0.0.0',
      port: 5173,
      headers: {
        'Content-Security-Policy': cspHeader,
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
