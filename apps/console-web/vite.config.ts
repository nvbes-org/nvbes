import path from 'node:path';
import tailwindcss from '@tailwindcss/vite';
import react from '@vitejs/plugin-react';
import devtoolsJson from 'vite-plugin-devtools-json';
import { defineConfig, loadEnv } from 'vite-plus';

export default defineConfig(({ mode }) => {
  const localEnv = loadEnv(mode, process.cwd(), '');
  const rootEnv = loadEnv(mode, path.resolve(process.cwd(), '../../'), '');
  const accountServiceProxyTarget =
    process.env.VITE_ACCOUNT_SERVICE_PROXY_TARGET ||
    localEnv.VITE_ACCOUNT_SERVICE_PROXY_TARGET ||
    rootEnv.VITE_ACCOUNT_SERVICE_PROXY_TARGET ||
    process.env.VITE_ACCOUNT_SERVICE_BASE_URL ||
    localEnv.VITE_ACCOUNT_SERVICE_BASE_URL ||
    rootEnv.VITE_ACCOUNT_SERVICE_BASE_URL ||
    'http://localhost:4000';

  return {
    plugins: [react(), tailwindcss(), devtoolsJson()],
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
          find: '@nvbes/identity-sdk',
          replacement: path.resolve(__dirname, '../../libs/ts/identity-sdk/src/index.ts'),
        },
        {
          find: '@nvbes/identity-sdk-web',
          replacement: path.resolve(__dirname, '../../libs/ts/identity-sdk-web/src/index.ts'),
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
      proxy: {
        '/auth': accountServiceProxyTarget,
        '/api': accountServiceProxyTarget,
        '/developer': accountServiceProxyTarget,
        '/oauth': accountServiceProxyTarget,
        '/.well-known': accountServiceProxyTarget,
      },
    },
  };
});
