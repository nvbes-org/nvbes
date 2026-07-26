import { defineConfig } from 'vite-plus';

export default defineConfig({
  pack: {
    entry: ['src/index.ts'],
    format: ['esm', 'cjs'],
    dts: {
      cjsReexport: true,
    },
    deps: {
      alwaysBundle: ['@nvbes/http-client'],
      onlyBundle: ['zod'],
    },
    outputOptions: {
      exports: 'named',
    },
    tsconfig: 'tsconfig.json',
  },
});
