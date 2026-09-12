import { defineConfig } from 'vite-plus';

export default defineConfig({
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
