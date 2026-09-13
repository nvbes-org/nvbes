#!/usr/bin/env node
import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs';
import { join } from 'node:path';

const errors = [];

function readJson(path) {
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

function checkProtoDocuments() {
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
}

function checkEventDocuments() {
  const manifest = readJson('contracts/events/manifest.json');
  const envelope = readJson('contracts/events/envelope.schema.json');
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

    const schema = readJson(event.schema);
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
    const schema = readJson(file);
    if (!schema) continue;
    if (!schema.$schema) {
      errors.push(`${file}: missing $schema identifier`);
    }
    if (schema.type !== 'object') {
      errors.push(`${file}: top-level schema type must be 'object'`);
    }
  }
}

checkProtoDocuments();
checkEventDocuments();

if (errors.length > 0) {
  console.error('Contract checks failed:');
  for (const error of errors) console.error(`- ${error}`);
  process.exit(1);
}

console.log('Contracts: ok');
