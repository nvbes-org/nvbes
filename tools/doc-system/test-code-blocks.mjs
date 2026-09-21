#!/usr/bin/env node
import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const REPO_ROOT = resolve(__dirname, '../..');

const errors = [];

function walk(dir, predicate, results = []) {
  if (!existsSync(dir)) return results;
  for (const entry of readdirSync(dir)) {
    const fullPath = join(dir, entry);
    const stat = statSync(fullPath);
    if (stat.isDirectory()) {
      if (entry !== 'node_modules' && entry !== 'dist' && entry !== '.git') {
        walk(fullPath, predicate, results);
      }
    } else if (predicate(fullPath)) {
      results.push(fullPath);
    }
  }
  return results;
}

function hasAngleTag(text) {
  let searchFrom = 0;
  let open = text.indexOf('<', searchFrom);
  while (open !== -1) {
    const close = text.indexOf('>', open + 1);
    if (close === -1) return false;
    const inner = text.slice(open + 1, close);
    if (inner.length > 0 && !/[\r\n\u2028\u2029]/.test(inner)) {
      return true;
    }
    searchFrom = open + 1;
    open = text.indexOf('<', searchFrom);
  }
  return false;
}

export function testCodeBlocks() {
  console.log('🧪 Running doc-testing on markdown code blocks...');

  const docFiles = walk(
    join(REPO_ROOT, 'docs/generated'),
    (f) => f.endsWith('.md') || f.endsWith('.mdx'),
  );
  let totalBlocks = 0;
  let jsonBlocks = 0;

  for (const file of docFiles) {
    const relPath = relative(REPO_ROOT, file);
    const content = readFileSync(file, 'utf8');
    const codeBlockRegex = /```([a-zA-Z0-9_-]+)?[ \t]*\r?\n([\s\S]*?)```/g;

    let match;
    let index = 0;
    while ((match = codeBlockRegex.exec(content)) !== null) {
      index++;
      totalBlocks++;
      const lang = (match[1] || '').toLowerCase().trim();
      const code = match[2].trim();

      if (!code) continue;

      // 1. Test JSON code blocks
      if (lang === 'json') {
        jsonBlocks++;
        // Ignore json blocks with placeholders like ... or <placeholder>
        const hasPlaceholders = code.includes('...') || hasAngleTag(code);
        if (!hasPlaceholders) {
          try {
            JSON.parse(code);
          } catch (e) {
            errors.push(`${relPath} (block #${index} [json]): Invalid JSON syntax: ${e.message}`);
          }
        }
      }

      // 2. Test balanced quotes in bash/sh code blocks (excluding comments)
      if (lang === 'bash' || lang === 'sh' || lang === 'shell') {
        const nonCommentLines = code
          .split('\n')
          .filter((line) => !line.trim().startsWith('#'))
          .join('\n');

        const doubleQuotes = (nonCommentLines.match(/"/g) || []).length;
        const singleQuotes = (nonCommentLines.match(/'/g) || []).length;
        if (doubleQuotes % 2 !== 0 && !nonCommentLines.includes('\\"')) {
          errors.push(
            `${relPath} (block #${index} [${lang}]): Unbalanced double quotes in shell block`,
          );
        }
        if (singleQuotes % 2 !== 0 && !nonCommentLines.includes("\\'")) {
          errors.push(
            `${relPath} (block #${index} [${lang}]): Unbalanced single quotes in shell block`,
          );
        }
      }
    }
  }

  if (errors.length > 0) {
    console.error('\n❌ Doc-testing failed:');
    for (const err of errors) {
      console.error(`  - ${err}`);
    }
    process.exit(1);
  }

  console.log(
    `✅ Doc-testing passed! (${totalBlocks} code blocks tested, including ${jsonBlocks} JSON schemas, 0 errors)`,
  );
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  testCodeBlocks();
}
