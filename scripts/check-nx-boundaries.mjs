#!/usr/bin/env node
import { execFileSync } from 'node:child_process';

function run(command, args) {
  return execFileSync(command, args, {
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  });
}

function parseJson(command, args) {
  return JSON.parse(run(command, args));
}

const projects = parseJson('./node_modules/.bin/nx', ['show', 'projects', '--json']);
const graph = parseJson('./node_modules/.bin/nx', ['graph', '--print']);

const projectMeta = new Map();
for (const name of projects) {
  const data = parseJson('./node_modules/.bin/nx', ['show', 'project', name, '--json']);
  projectMeta.set(name, {
    tags: new Set(Array.isArray(data.tags) ? data.tags : []),
    projectType: data.projectType,
  });
}

function tag(meta, prefix) {
  return [...meta.tags].find((value) => value.startsWith(`${prefix}:`))?.slice(prefix.length + 1);
}

function getDomain(meta) {
  return tag(meta, 'domain');
}

function getLayer(meta) {
  return tag(meta, 'layer');
}

function errorsForProject(_name, meta) {
  const errors = [];
  const type = tag(meta, 'type');
  const domain = getDomain(meta);
  const layer = getLayer(meta);

  if (!type) errors.push(`missing type tag`);
  if (!domain) errors.push(`missing domain tag`);
  if (!layer) errors.push(`missing layer tag`);

  if (type === 'app' && layer !== 'app') {
    errors.push(`app projects must use layer:app`);
  }

  if (type === 'lib' && layer === 'app') {
    errors.push(`library projects cannot use layer:app`);
  }

  if (layer === 'core' && domain !== 'shared') {
    errors.push(`core layer must use domain:shared`);
  }

  return errors;
}

const dependencyErrors = [];
const depsBySource = graph.graph.dependencies ?? {};

for (const [name, meta] of projectMeta.entries()) {
  for (const error of errorsForProject(name, meta)) {
    dependencyErrors.push(`${name}: ${error}`);
  }
}

for (const [source, deps] of Object.entries(depsBySource)) {
  const sourceMeta = projectMeta.get(source);
  if (!sourceMeta) continue;

  for (const dep of deps) {
    const targetMeta = projectMeta.get(dep.target);
    if (!targetMeta) continue;

    const sourceDomain = getDomain(sourceMeta);
    const targetDomain = getDomain(targetMeta);
    const sourceLayer = getLayer(sourceMeta);
    const targetLayer = getLayer(targetMeta);

    if (source === dep.target) {
      dependencyErrors.push(`${source} depends on itself`);
      continue;
    }

    if (sourceLayer === 'core') {
      dependencyErrors.push(`${source} is core and must not depend on ${dep.target}`);
      continue;
    }

    if (sourceDomain === 'drive' && targetDomain === 'identity') {
      dependencyErrors.push(`${source} cannot depend on identity project ${dep.target}`);
    }

    if (sourceDomain === 'identity' && targetDomain === 'drive') {
      dependencyErrors.push(`${source} cannot depend on drive project ${dep.target}`);
    }

    if (sourceLayer === 'app' && targetLayer === 'app') {
      dependencyErrors.push(`${source} cannot depend on app project ${dep.target}`);
    }

    if (targetLayer === 'app' && sourceLayer !== 'app') {
      dependencyErrors.push(`${source} cannot depend on app project ${dep.target}`);
    }

    if (sourceLayer === 'sdk' && targetLayer === 'app') {
      dependencyErrors.push(`${source} cannot depend on app project ${dep.target}`);
    }

    if (sourceDomain === 'shared' && targetDomain && targetDomain !== 'shared') {
      dependencyErrors.push(`${source} cannot depend on domain-specific project ${dep.target}`);
    }
  }
}

if (dependencyErrors.length > 0) {
  console.error('Nx dependency boundary violations:');
  for (const error of dependencyErrors) {
    console.error(`- ${error}`);
  }
  process.exit(1);
}

console.log('Nx dependency boundaries: ok');
