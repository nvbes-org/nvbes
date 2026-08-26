#!/usr/bin/env node
import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { testCodeBlocks } from './test-code-blocks.mjs';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const REPO_ROOT = resolve(__dirname, '../..');

const errors = [];
const warnings = [];

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

/**
 * Basic syntax and sanity checker for Mermaid blocks in markdown
 */
function validateMermaidInFile(filePath) {
  const content = readFileSync(filePath, 'utf8');
  const relPath = relative(REPO_ROOT, filePath);
  const mermaidRegex = /```mermaid\s*([\s\S]*?)```/g;

  let match;
  let blockIndex = 0;
  while ((match = mermaidRegex.exec(content)) !== null) {
    blockIndex++;
    const code = match[1].trim();
    if (!code) {
      errors.push(`${relPath} (block #${blockIndex}): empty mermaid diagram`);
      continue;
    }

    const firstLine = code.split('\n')[0].trim();
    const validDiagramTypes = [
      'flowchart',
      'graph',
      'sequenceDiagram',
      'classDiagram',
      'stateDiagram',
      'stateDiagram-v2',
      'erDiagram',
      'gantt',
      'pie',
      'gitGraph',
      'mindmap',
      'timeline',
      'c4Context',
    ];

    const hasValidType = validDiagramTypes.some((t) => firstLine.startsWith(t));
    if (!hasValidType) {
      errors.push(
        `${relPath} (block #${blockIndex}): invalid mermaid diagram type on line 1: "${firstLine}"`,
      );
    }

    // Check for unbalanced braces or quotes
    const openBraces = (code.match(/\{/g) || []).length;
    const closeBraces = (code.match(/\}/g) || []).length;
    if (openBraces !== closeBraces) {
      errors.push(
        `${relPath} (block #${blockIndex}): unbalanced curly braces in mermaid block ({: ${openBraces}, }: ${closeBraces})`,
      );
    }

    const openSubgraphs = (code.match(/\bsubgraph\b/g) || []).length;
    const endSubgraphs = (code.match(/\bend\b/g) || []).length;
    if (openSubgraphs > 0 && openSubgraphs !== endSubgraphs) {
      errors.push(
        `${relPath} (block #${blockIndex}): unbalanced subgraphs in mermaid block (subgraph: ${openSubgraphs}, end: ${endSubgraphs})`,
      );
    }

    // Sequence diagram specific rules
    if (firstLine.startsWith('sequenceDiagram')) {
      const lines = code.split('\n');
      for (let i = 0; i < lines.length; i++) {
        const line = lines[i].trim();
        // Check for unquoted participant/actor alias with spaces or special characters
        const aliasMatch = line.match(/^(?:participant|actor)\s+([A-Za-z0-9_-]+)\s+as\s+(.+)$/);
        if (aliasMatch) {
          const label = aliasMatch[2].trim();
          if (
            (label.includes(' ') || label.includes('(') || label.includes('/')) &&
            !label.startsWith('"')
          ) {
            errors.push(
              `${relPath} (block #${blockIndex}, line ${i + 1}): participant/actor label with spaces or parentheses must be double-quoted: "${line}"`,
            );
          }
        }
      }
    }
  }
}

/**
 * Validate frontmatter in documentation pages
 */
function validateFrontmatter(filePath) {
  const content = readFileSync(filePath, 'utf8');
  const relPath = relative(REPO_ROOT, filePath);

  if (!content.startsWith('---')) {
    warnings.push(`${relPath}: missing frontmatter (---)`);
    return;
  }

  const endIdx = content.indexOf('---', 3);
  if (endIdx === -1) {
    errors.push(`${relPath}: unclosed frontmatter header`);
    return;
  }

  const fm = content.slice(3, endIdx);
  if (!fm.includes('title:')) {
    errors.push(`${relPath}: frontmatter missing required "title" property`);
  }
}

/**
 * Check relative markdown links and Starlight web routes
 */
function validateLinksInFile(filePath) {
  const content = readFileSync(filePath, 'utf8');
  const relPath = relative(REPO_ROOT, filePath);
  const dir = dirname(filePath);
  const docsRoot = join(REPO_ROOT, 'docs/generated');

  const linkRegex = /\[([^\]]+)\]\(([^)]+)\)/g;
  let match;
  while ((match = linkRegex.exec(content)) !== null) {
    const rawTarget = match[2].trim();
    if (
      rawTarget.startsWith('http://') ||
      rawTarget.startsWith('https://') ||
      rawTarget.startsWith('#') ||
      rawTarget.startsWith('mailto:')
    ) {
      continue;
    }

    const cleanTarget = rawTarget.split('#')[0].split('?')[0];
    if (!cleanTarget) continue;

    if (cleanTarget.startsWith('/')) {
      // Starlight root-relative route
      const cleanRoute = cleanTarget.replace(/\/$/, '');
      const possiblePaths = [
        join(docsRoot, `${cleanRoute}.md`),
        join(docsRoot, `${cleanRoute}.mdx`),
        join(docsRoot, `${cleanRoute}/index.md`),
        join(docsRoot, `${cleanRoute}/index.mdx`),
      ];
      const exists = possiblePaths.some((p) => existsSync(p));
      if (!exists) {
        warnings.push(`${relPath}: route link points to non-existent document "${rawTarget}"`);
      }
    } else {
      const resolved = resolve(dir, cleanTarget);
      if (!existsSync(resolved)) {
        warnings.push(`${relPath}: relative link points to non-existent file "${rawTarget}"`);
      }
    }
  }
}

export function validateDocumentation() {
  console.log('🛡️ Running documentation quality gates & guardrails...');

  const docFiles = [
    ...walk(join(REPO_ROOT, 'docs/generated'), (f) => f.endsWith('.md') || f.endsWith('.mdx')),
  ];

  if (docFiles.length === 0) {
    warnings.push('No generated documentation files found in docs/generated yet.');
  }

  for (const file of docFiles) {
    validateMermaidInFile(file);
    validateFrontmatter(file);
    validateLinksInFile(file);
  }

  // Run code blocks / snippets validator
  testCodeBlocks();

  if (warnings.length > 0) {
    console.warn('\n⚠️ Documentation warnings:');
    for (const w of warnings) {
      console.warn(`  - ${w}`);
    }
  }

  if (errors.length > 0) {
    console.error('\n❌ Documentation validation failed with errors:');
    for (const e of errors) {
      console.error(`  - ${e}`);
    }
    process.exit(1);
  }

  console.log(`✅ Documentation validation passed! (${docFiles.length} files checked, 0 errors)`);
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  validateDocumentation();
}
