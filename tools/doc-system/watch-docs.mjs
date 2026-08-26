#!/usr/bin/env node
import { watch } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { fixAllDocumentation } from './fix-docs.mjs';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const REPO_ROOT = resolve(__dirname, '../..');

const WATCH_TARGETS = [join(REPO_ROOT, 'apps'), join(REPO_ROOT, 'libs'), join(REPO_ROOT, 'docs')];

let debounceTimer = null;
const DEBOUNCE_MS = 600;

function triggerRegeneration(eventType, filename, targetDir) {
  if (!filename) return;
  // Ignore irrelevant files
  if (
    filename.includes('node_modules') ||
    filename.includes('.git') ||
    filename.includes('dist') ||
    filename.includes('target') ||
    filename.startsWith('generated/') ||
    filename.startsWith('.docgen') ||
    filename.endsWith('.tmp')
  ) {
    return;
  }

  // Only react to architecture/code/doc files
  const relevantExtensions = ['.rs', '.ts', '.tsx', '.json', '.md', '.mdx', '.toml'];
  const isRelevant = relevantExtensions.some((ext) => filename.endsWith(ext));
  if (!isRelevant) return;

  if (debounceTimer) {
    clearTimeout(debounceTimer);
  }

  debounceTimer = setTimeout(() => {
    console.log(
      `\n🔄 Change detected in [${eventType}] ${filename}. Auto-regenerating documentation...`,
    );
    try {
      fixAllDocumentation();
    } catch (err) {
      console.error(`⚠️ Error during auto-regeneration: ${err.message}`);
    }
  }, DEBOUNCE_MS);
}

export function startWatchMode() {
  console.log('👀 Documentation Watch Mode active. Monitoring codebase for changes...');
  console.log(`📂 Watching: apps/, libs/, docs/\n`);

  // Run initial sync
  fixAllDocumentation();

  for (const target of WATCH_TARGETS) {
    try {
      watch(target, { recursive: true }, (eventType, filename) => {
        triggerRegeneration(eventType, filename, target);
      });
    } catch (err) {
      console.warn(`[WARN] Could not watch ${target}: ${err.message}`);
    }
  }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  startWatchMode();
}
