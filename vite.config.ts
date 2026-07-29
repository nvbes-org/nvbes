import { defineConfig } from 'vite-plus';

const ignoredGeneratedAndBuildOutputs = [
  '**/dist/**',
  '**/target/**',
  '**/node_modules/**',
  '**/.git/**',
  '**/test-results/**',
  '**/playwright-report/**',
  '**/apps/*/migrations/**',
  '**/libs/ts/identity-sdk-core/src/types.gen.ts',
  '**/openapi.json',
];

export default defineConfig({
  fmt: {
    ignorePatterns: ignoredGeneratedAndBuildOutputs,
    printWidth: 100,
    tabWidth: 2,
    singleQuote: true,
    semi: true,
    trailingComma: 'all',
  },
  lint: {
    ignorePatterns: ignoredGeneratedAndBuildOutputs,
    options: {
      typeAware: true,
      typeCheck: true,
    },
    rules: {
      'typescript/no-explicit-any': 'error',
    },
  },
  run: {
    enablePrePostScripts: true,
  },
  staged: {
    '*.{js,ts,cjs,mjs,jsx,tsx,json,jsonc,css,scss,md,html}': 'vp check --fix',
  },
});
