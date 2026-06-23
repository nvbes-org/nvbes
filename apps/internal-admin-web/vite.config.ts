import path from 'node:path';
import tailwindcss from '@tailwindcss/vite';
import react from '@vitejs/plugin-react';
import devtoolsJson from 'vite-plugin-devtools-json';
import { defineConfig, loadEnv, type Plugin } from 'vite-plus';

const permissionsPolicy =
  'accelerometer=(), camera=(), geolocation=(), gyroscope=(), magnetometer=(), microphone=(), payment=(), usb=()';

function cspFor(mode: string): string {
  const connectSrc =
    mode === 'development'
      ? "connect-src 'self' ws://localhost:* http://localhost:*;"
      : "connect-src 'self';";
  return `default-src 'self'; script-src 'self' ${mode === 'development' ? "'unsafe-inline' 'unsafe-eval'" : ''}; worker-src 'self' blob:; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; font-src 'self' data:; ${connectSrc} object-src 'none'; base-uri 'self'; form-action 'self'; frame-ancestors 'none'; upgrade-insecure-requests;`;
}

function cspPlugin(mode: string): Plugin {
  return {
    name: 'internal-admin-csp',
    transformIndexHtml(html: string) {
      const csp = cspFor(mode).replace("; frame-ancestors 'none'", '');
      return html.replace(
        '<!-- %CSP_META% -->',
        `<meta http-equiv="Content-Security-Policy" content="${csp}" />`,
      );
    },
  };
}

export default defineConfig(({ mode }) => {
  const localEnv = loadEnv(mode, process.cwd(), '');
  const rootEnv = loadEnv(mode, path.resolve(process.cwd(), '../../'), '');
  const internalAdminApiBaseUrl =
    process.env.VITE_INTERNAL_ADMIN_API_BASE_URL ||
    localEnv.VITE_INTERNAL_ADMIN_API_BASE_URL ||
    rootEnv.VITE_INTERNAL_ADMIN_API_BASE_URL ||
    'http://localhost:4000';
  const cspHeader = cspFor(mode);

  return {
    plugins: [tailwindcss(), react(), devtoolsJson(), cspPlugin(mode)],
    build: {
      target: 'esnext',
      sourcemap: true,
      minify: true,
      cssMinify: 'esbuild',
      manifest: true,
      modulePreload: { polyfill: false },
    },
    resolve: {
      alias: [
        { find: '@', replacement: path.resolve(__dirname, './src') },
        {
          find: '@nvbes/web-runtime',
          replacement: path.resolve(__dirname, '../../libs/ts/web-runtime/src/index.ts'),
        },
      ],
    },
    server: {
      port: 5178,
      host: 'localhost',
      headers: {
        'Content-Security-Policy': cspHeader,
        'X-Frame-Options': 'DENY',
        'X-Content-Type-Options': 'nosniff',
        'Referrer-Policy': 'strict-origin-when-cross-origin',
        'Permissions-Policy': permissionsPolicy,
      },
      proxy: {
        '/admin': { target: internalAdminApiBaseUrl, changeOrigin: true },
        '/workspaces': { target: internalAdminApiBaseUrl, changeOrigin: true },
      },
    },
    preview: {
      port: 5178,
      host: 'localhost',
      headers: {
        'Content-Security-Policy': cspHeader,
        'X-Frame-Options': 'DENY',
        'X-Content-Type-Options': 'nosniff',
        'Referrer-Policy': 'strict-origin-when-cross-origin',
        'Permissions-Policy': permissionsPolicy,
      },
    },
  };
});
