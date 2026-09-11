import { defineConfig } from 'vite-plus';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';
import { fileURLToPath } from 'node:url';
import { hostedDocument, identityBackend } from './dev.routing';

const backend = identityBackend(process.env.IDENTITY_WEB_BACKEND_URL);

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: { alias: { '@': fileURLToPath(new URL('./src', import.meta.url)) } },
  server: {
    host: '127.0.0.1',
    port: 4200,
    strictPort: true,
    headers: {
      'Cache-Control': 'no-store',
      'Referrer-Policy': 'no-referrer',
      'X-Content-Type-Options': 'nosniff',
    },
    proxy: backend
      ? {
          '/oauth': {
            target: backend,
            changeOrigin: false,
            bypass: (request) => (hostedDocument(request) ? '/index.html' : undefined),
          },
        }
      : undefined,
  },
  test: { environment: 'jsdom', include: ['src/**/*.test.ts', 'src/**/*.test.tsx'] },
});
