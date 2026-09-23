import { defineConfig } from 'vite-plus';

const ignoredArchivedGeneratedAndBuildOutputs = [
  '**/archive/**',
  '**/dist/**',
  '**/target/**',
  '**/node_modules/**',
  '**/.git/**',
  '**/test-results/**',
  '**/playwright-report/**',
  '**/apps/*/migrations/**',
  '**/libs/ts/identity-sdk-core/src/types.gen.ts',
  '**/libs/ts/account-sdk-core/src/types.gen.ts',
  '**/openapi.json',
];

export default defineConfig({
  fmt: {
    ignorePatterns: ignoredArchivedGeneratedAndBuildOutputs,
    printWidth: 100,
    tabWidth: 2,
    singleQuote: true,
    semi: true,
    trailingComma: 'all',
  },
  lint: {
    ignorePatterns: ignoredArchivedGeneratedAndBuildOutputs,
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
