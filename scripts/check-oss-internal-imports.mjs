#!/usr/bin/env node
import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';

const roots = ['apps', 'libs', 'tools'];
const textExtensions = new Set(['.js', '.jsx', '.mjs', '.rs', '.ts', '.tsx']);
const skippedDirs = new Set(['.git', '.nx', 'coverage', 'dist', 'node_modules', 'target']);
const importPatterns = [
  /\bfrom\s+['"]([^'"]+)['"]/g,
  /\bimport\s*\(\s*['"]([^'"]+)['"]\s*\)/g,
  /\brequire\s*\(\s*['"]([^'"]+)['"]\s*\)/g,
  /#\[path\s*=\s*"([^"]+)"\]/g,
  /\binclude(?:_str|_bytes)?!\s*\(\s*"([^"]+)"\s*\)/g,
];

function normalizePath(value) {
  return value.replaceAll('\\', '/').replace(/\/+$/, '');
}

function readJson(path) {
  return JSON.parse(readFileSync(path, 'utf8'));
}

function tag(meta, prefix) {
  return [...meta.tags].find((value) => value.startsWith(`${prefix}:`))?.slice(prefix.length + 1);
}

function projectFiles(dir, files = []) {
  if (!existsSync(dir)) return files;
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) {
      if (!skippedDirs.has(entry.name)) projectFiles(path, files);
    } else if (entry.name === 'project.json') {
      files.push(path);
    }
  }
  return files;
}

function localPackageName(root) {
  const path = join(root, 'package.json');
  return existsSync(path) ? readJson(path).name : undefined;
}

function isTextFile(path) {
  const basename = path.split('/').at(-1) ?? '';
  const dot = basename.lastIndexOf('.');
  return dot >= 0 && textExtensions.has(basename.slice(dot));
}

function walk(dir, files = []) {
  if (!existsSync(dir)) return files;
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    if (skippedDirs.has(entry.name)) continue;
    const path = join(dir, entry.name);
    if (entry.isDirectory()) {
      walk(path, files);
    } else {
      const relativePath = normalizePath(relative(process.cwd(), path));
      if (isTextFile(relativePath)) files.push(relativePath);
    }
  }
  return files;
}

function importedSpecifiers(content) {
  const specifiers = [];
  for (const pattern of importPatterns) {
    pattern.lastIndex = 0;
    for (const match of content.matchAll(pattern)) specifiers.push(match[1]);
  }
  return specifiers;
}

function resolvesInside(specifier, sourceFile, targetRoot) {
  if (!specifier.startsWith('.')) return false;
  const resolved = normalizePath(join(dirname(sourceFile), specifier));
  return resolved === targetRoot || resolved.startsWith(`${targetRoot}/`);
}

const projectMeta = [];
const packageToInternalProject = new Map();

for (const path of roots.flatMap((root) => projectFiles(root))) {
  const root = normalizePath(dirname(path));
  const data = readJson(path);
  const meta = {
    name: data.name,
    root,
    sourceRoot: normalizePath(data.sourceRoot ?? root),
    tags: new Set(Array.isArray(data.tags) ? data.tags : []),
  };
  projectMeta.push(meta);

  if (tag(meta, 'scope') === 'internal') {
    const packageName = localPackageName(root);
    if (packageName) packageToInternalProject.set(packageName, meta.name);
  }
}

const internalProjects = projectMeta.filter((meta) => tag(meta, 'scope') === 'internal');
const sourceProjects = projectMeta.filter((meta) => ['oss', 'cloud'].includes(tag(meta, 'scope')));
const errors = [];

for (const sourceProject of sourceProjects) {
  for (const file of walk(sourceProject.sourceRoot)) {
    const content = readFileSync(file, 'utf8');
    for (const specifier of importedSpecifiers(content)) {
      const internalPackage = packageToInternalProject.get(specifier);
      if (internalPackage) {
        errors.push(`${file}: ${sourceProject.name} imports internal package ${specifier} (${internalPackage})`);
      }

      for (const target of internalProjects) {
        const directRoot = specifier === target.root || specifier.startsWith(`${target.root}/`);
        if (directRoot || resolvesInside(specifier, file, target.root)) {
          errors.push(`${file}: ${sourceProject.name} imports internal project path ${target.root}`);
        }
      }
    }
  }
}

if (errors.length > 0) {
  console.error('OSS/internal source import violations:');
  for (const error of errors) console.error(`- ${error}`);
  process.exit(1);
}

console.log('OSS/internal source import boundaries: ok');
