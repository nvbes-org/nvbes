import path from 'node:path';
import tailwindcss from '@tailwindcss/vite';
import react from '@vitejs/plugin-react';
import { visualizer } from 'rollup-plugin-visualizer';
import devtoolsJson from 'vite-plugin-devtools-json';
import { VitePWA } from 'vite-plugin-pwa';
import { defineConfig, loadEnv, type Plugin, type PluginOption } from 'vite-plus';
import { observabilitySourceMapPlugins } from '../../tools/web-build/vite-observability-sourcemaps';
import { sriPlugin } from '../../tools/web-build/vite-sri';
import {
  buildWebCsp,
  cspMetaFromHeader,
  integrityPolicyScripts,
  permissionsPolicy,
  uaClientHintsHeaders,
} from '../../libs/ts/web-runtime/src/csp';

function getCsp(mode: string): string {
  const isDev = mode === 'development';

  return buildWebCsp({
    mode,
    styleSrc: ['https://fonts.googleapis.com'],
    imgSrc: ['https:'],
    fontSrc: ['https://fonts.gstatic.com'],
    connectSrc: isDev ? ['http://localhost:8080', 'http://127.0.0.1:8080'] : [],
  });
}

function getMetaCsp(mode: string): string {
  return cspMetaFromHeader(getCsp(mode));
}

function pluginList(plugin: unknown): PluginOption[] {
  return Array.isArray(plugin) ? (plugin as PluginOption[]) : [plugin as PluginOption];
}

function cspPlugin(mode: string): Plugin {
  return {
    name: 'csp-injection-plugin',
    transformIndexHtml(html: string) {
      const cspString = getMetaCsp(mode);
      const metaTag = `<meta http-equiv="Content-Security-Policy" content="${cspString}" />`;
      return html.replace('<!-- %CSP_META% -->', metaTag);
    },
  };
}

export default defineConfig(({ mode }) => {
  const localEnv = loadEnv(mode, process.cwd(), '');
  const rootEnv = loadEnv(mode, path.resolve(process.cwd(), '../../'), '');
  const envSources = [process.env, localEnv, rootEnv];

  const configuredCloudServiceBaseUrl =
    process.env.VITE_CLOUD_SERVICE_BASE_URL ||
    localEnv.VITE_CLOUD_SERVICE_BASE_URL ||
    rootEnv.VITE_CLOUD_SERVICE_BASE_URL ||
    '';
  const cloudServiceProxyTarget =
    process.env.VITE_CLOUD_SERVICE_PROXY_TARGET ||
    localEnv.VITE_CLOUD_SERVICE_PROXY_TARGET ||
    rootEnv.VITE_CLOUD_SERVICE_PROXY_TARGET ||
    process.env.NVBES_CLOUD_SERVICE_BASE_URL ||
    localEnv.NVBES_CLOUD_SERVICE_BASE_URL ||
    rootEnv.NVBES_CLOUD_SERVICE_BASE_URL ||
    (configuredCloudServiceBaseUrl.startsWith('http') ? configuredCloudServiceBaseUrl : '') ||
    'http://localhost:4002';

  const cspHeader = getCsp(mode);

  return {
    plugins: [
      ...pluginList(react()),
      ...pluginList(tailwindcss()),
      devtoolsJson(),
      cspPlugin(mode),
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
          includeAssets: [
            'icon.svg',
            'icon-180.png',
            'icon-192.png',
            'icon-512.png',
            'offline.html',
          ],
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
      ...observabilitySourceMapPlugins({ appName: 'cloud-web', envSources }),
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
          find: '@nvbes/identity-sdk',
          replacement: path.resolve(__dirname, '../../libs/ts/identity-sdk/src/index.ts'),
        },
        {
          find: '@nvbes/web-ui',
          replacement: path.resolve(__dirname, '../../libs/ts/web-ui/src/index.ts'),
        },
        {
          find: '@nvbes/web-runtime/analytics',
          replacement: path.resolve(__dirname, '../../libs/ts/web-runtime/src/analytics.ts'),
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
      strictPort: true,
      headers: {
        ...uaClientHintsHeaders,
        'Content-Security-Policy': cspHeader,
        'Integrity-Policy-Report-Only': integrityPolicyScripts,
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
          target: cloudServiceProxyTarget,
          changeOrigin: true,
          rewrite: (requestPath: string) => requestPath.replace(/^\/api/u, ''),
        },
      },
    },
    preview: {
      host: '0.0.0.0',
      port: 5173,
      strictPort: true,
      headers: {
        ...uaClientHintsHeaders,
        'Content-Security-Policy': cspHeader,
        'Integrity-Policy-Report-Only': integrityPolicyScripts,
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
