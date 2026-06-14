import path from 'node:path';
import tailwindcss from '@tailwindcss/vite';
import react from '@vitejs/plugin-react';
import devtoolsJson from 'vite-plugin-devtools-json';
import { defineConfig, loadEnv } from 'vite-plus';

export default defineConfig(({ mode }) => {
  const localEnv = loadEnv(mode, process.cwd(), '');
  const rootEnv = loadEnv(mode, '../../', '');
  const identityApiProxyTarget =
    process.env.VITE_IDENTITY_API_PROXY_TARGET ||
    localEnv.VITE_IDENTITY_API_PROXY_TARGET ||
    rootEnv.VITE_IDENTITY_API_PROXY_TARGET ||
    'http://localhost:8080';

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
          find: '@nvbes/identity-sdk-web',
          replacement: path.resolve(__dirname, '../../libs/ts/identity-sdk-web/src/index.ts'),
        },
        {
          find: '@nvbes/web-runtime/posthog',
          replacement: path.resolve(__dirname, '../../libs/ts/web-runtime/src/posthog.ts'),
        },
        {
          find: '@nvbes/web-runtime',
          replacement: path.resolve(__dirname, '../../libs/ts/web-runtime/src/index.ts'),
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
      port: 5175,
      proxy: {
        '^/api(?:/|$)': identityApiProxyTarget,
        '/developer': identityApiProxyTarget,
        '/oauth': identityApiProxyTarget,
      },
    },
  };
});
