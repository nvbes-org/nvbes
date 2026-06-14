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
    server: {
      port: 5175,
      proxy: {
        '/api': identityApiProxyTarget,
        '/oauth': identityApiProxyTarget,
        '/.well-known': identityApiProxyTarget
      }
    }
  };
});
