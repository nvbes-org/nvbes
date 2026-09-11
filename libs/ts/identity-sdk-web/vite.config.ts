import { defineConfig } from 'vite-plus';

export default defineConfig({
  resolve: {
    alias: {
      '@nvbes/http-client': new URL('../http-client/src/index.ts', import.meta.url).pathname,
    },
  },
  pack: {
    entry: ['src/index.ts', 'src/oauth.ts'],
    format: ['esm'],
    dts: true,
  },
  test: {
    globals: true,
    environment: 'node',
    include: ['src/**/*.test.ts', 'src/**/*.test.tsx'],
  },
});
