#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs';
import { join } from 'node:path';

function readJson(path, errors) {
  if (!existsSync(path)) {
    errors.push(`${path}: missing`);
    return undefined;
  }
  try {
    return JSON.parse(readFileSync(path, 'utf8'));
  } catch (error) {
    errors.push(`${path}: invalid JSON: ${error.message}`);
    return undefined;
  }
}

function walk(dir, predicate, results = []) {
  if (!existsSync(dir)) return results;
  for (const entry of readdirSync(dir)) {
    const path = join(dir, entry);
    const stat = statSync(path);
    if (stat.isDirectory()) walk(path, predicate, results);
    else if (predicate(path)) results.push(path);
  }
  return results;
}

export function checkProtoDocuments(errors = []) {
  const files = walk('contracts/protobuf', (path) => path.endsWith('.proto'));
  if (files.length === 0) errors.push('contracts/protobuf: at least one .proto file is required');
  for (const file of files) {
    const content = readFileSync(file, 'utf8');
    if (!content.includes('syntax = "proto3";')) errors.push(`${file}: proto3 syntax is required`);
    if (!/^package\s+nvbes\.[a-z0-9_.]+\.v\d+;/m.test(content)) {
      errors.push(`${file}: package must be versioned under nvbes.*.vN`);
    }
    if (/\bTODO\b|\bTBD\b|\bFIXME\b/i.test(content)) errors.push(`${file}: unresolved marker`);
  }
  return errors;
}

export function checkEventDocuments(errors = []) {
  const manifest = readJson('contracts/events/manifest.json', errors);
  const envelope = readJson('contracts/events/envelope.schema.json', errors);
  if (!manifest || !envelope) return;

  for (const field of [
    'event_id',
    'event_type',
    'event_version',
    'tenant_id',
    'region_id',
    'occurred_at',
    'correlation_id',
    'idempotency_key',
    'payload',
  ]) {
    if (!envelope.required?.includes(field)) {
      errors.push(`contracts/events/envelope.schema.json: missing required field ${field}`);
    }
  }

  if (!Array.isArray(manifest.events) || manifest.events.length === 0) {
    errors.push('contracts/events/manifest.json: events must be a non-empty array');
    return;
  }

  const seen = new Set();
  for (const event of manifest.events) {
    const key = `${event.event_type}@${event.event_version}`;
    if (seen.has(key)) errors.push(`contracts/events/manifest.json: duplicate event ${key}`);
    seen.add(key);
    if (!event.critical) {
      errors.push(`${key}: critical=true is required while the event remains in the manifest`);
    }

    const schema = readJson(event.schema, errors);
    if (!schema) continue;
    if (schema.properties?.event_type?.const !== event.event_type) {
      errors.push(`${event.schema}: event_type const must match manifest`);
    }
    if (schema.properties?.event_version?.const !== event.event_version) {
      errors.push(`${event.schema}: event_version const must match manifest`);
    }
    if (!schema.properties?.payload || !schema.required?.includes('payload')) {
      errors.push(`${event.schema}: payload property is required`);
    }
  }

  const allSchemas = walk('contracts/events', (path) => path.endsWith('.schema.json'));
  for (const file of allSchemas) {
    const schema = readJson(file, errors);
    if (!schema) continue;
    if (!schema.$schema) {
      errors.push(`${file}: missing $schema identifier`);
    }
    if (schema.type !== 'object') {
      errors.push(`${file}: top-level schema type must be 'object'`);
    }
  }
  return errors;
}

export function checkProtoBreaking(errors = [], env = process.env) {
  const against = env.NVBES_BUF_BREAKING_AGAINST;
  let target = against;
  if (!target && env.CI !== 'true') {
    for (const candidate of ['origin/main', 'main']) {
      const check = spawnSync('git', ['rev-parse', '--verify', candidate], { encoding: 'utf8' });
      if (check.status === 0) {
        target = `.git#branch=${candidate},subdir=contracts/protobuf`;
        break;
      }
    }
  }
  if (!target) return errors;

  const buf = spawnSync('buf', ['breaking', 'contracts/protobuf', '--against', target], {
    encoding: 'utf8',
  });
  if (buf.status !== 0 && buf.status !== null) {
    errors.push(
      `contracts/protobuf: breaking changes detected against ${target}:\n${buf.stderr || buf.stdout}`,
    );
  }
  return errors;
}

export function runContractChecks(env = process.env) {
  const errors = [];
  checkProtoDocuments(errors);
  checkProtoBreaking(errors, env);
  checkEventDocuments(errors);
  return errors;
}

import { pathToFileURL } from 'node:url';

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const errors = runContractChecks();
  if (errors.length > 0) {
    console.error('Contract checks failed:');
    for (const error of errors) console.error(`- ${error}`);
    process.exit(1);
  }
  console.log('Contracts: ok');
}
