import { defineConfig } from 'vite-plus';

export default defineConfig({
  resolve: {
    alias: {
      '@nvbes/http-client': new URL('../http-client/src/index.ts', import.meta.url).pathname,
    },
  },
  pack: {
    entry: ['src/index.ts'],
    format: ['esm'],
    dts: true,
  },
  test: {
    environment: 'node',
    include: ['src/**/*.test.ts'],
  },
});
