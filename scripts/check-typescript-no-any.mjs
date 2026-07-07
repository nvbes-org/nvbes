#!/usr/bin/env node
import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';
import ts from 'typescript';

const roots = ['apps', 'libs/ts'];
const skippedDirs = new Set(['.git', '.nx', 'coverage', 'dist', 'node_modules']);
const sourceExtensions = /\.(?:d\.)?tsx?$/u;
const failures = [];

for (const root of roots) {
  for (const file of walk(root)) {
    const text = readFileSync(file, 'utf8');
    const sourceFile = ts.createSourceFile(file, text, ts.ScriptTarget.Latest, true);
    inspectNode(sourceFile, sourceFile);
  }
}

if (failures.length > 0) {
  console.error('TypeScript explicit any check failed:');
  for (const failure of failures) {
    console.error(`- ${failure.path}:${failure.line}:${failure.column} explicit any is forbidden; use unknown or a precise type`);
  }
  process.exit(1);
}

console.log('TypeScript explicit any check passed.');

function walk(dir, files = []) {
  if (!existsSync(dir)) return files;
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    if (entry.isDirectory()) {
      if (!skippedDirs.has(entry.name)) walk(join(dir, entry.name), files);
      continue;
    }

    const path = join(dir, entry.name);
    if (sourceExtensions.test(path)) files.push(path);
  }
  return files;
}

function inspectNode(node, sourceFile) {
  if (node.kind === ts.SyntaxKind.AnyKeyword) {
    const position = sourceFile.getLineAndCharacterOfPosition(node.getStart(sourceFile));
    failures.push({
      column: position.character + 1,
      line: position.line + 1,
      path: normalizePath(relative(process.cwd(), sourceFile.fileName)),
    });
  }
  ts.forEachChild(node, (child) => inspectNode(child, sourceFile));
}

function normalizePath(path) {
  return path.replaceAll('\\', '/');
}
