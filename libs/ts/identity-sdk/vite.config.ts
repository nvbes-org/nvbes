import { defineConfig } from 'vite-plus';

export default defineConfig({
  resolve: {
    alias: {
      '@nvbes/http-client': new URL('../http-client/src/index.ts', import.meta.url).pathname,
    },
  },
  pack: {
    entry: ['src/index.ts'],
    format: ['esm', 'cjs'],
    dts: true,
    deps: {
      alwaysBundle: ['@nvbes/http-client'],
      onlyBundle: ['zod'],
    },
    outputOptions: {
      exports: 'named',
    },
    tsconfig: 'tsconfig.json',
  },
  test: {
    environment: 'node',
    include: ['src/**/*.test.js', 'src/**/*.test.ts'],
  },
});
