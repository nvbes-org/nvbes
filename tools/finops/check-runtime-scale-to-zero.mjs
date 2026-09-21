#!/usr/bin/env node

import { readFile } from 'node:fs/promises';
import path from 'node:path';

/**
 * Runtime scale-to-zero verifier.
 * Validates that production container definitions and serverless database configs
 * strictly mandate zero-idle scale, ensuring that costs drop to 0€ when quiescent.
 */

export async function verifyRuntimeScaleToZero(configPath) {
  const resolved = path.resolve(process.cwd(), configPath);
  const rel = path.relative(process.cwd(), resolved);
  if (rel.startsWith('..') || path.isAbsolute(rel)) {
    throw new Error(`Invalid path: ${configPath}`);
  }
  const content = await readFile(resolved, 'utf8');

  const violations = [];

  // Match scaleway_container blocks
  const containerMinScaleMatches = content.matchAll(/min_scale\s*=\s*([^\s\n]+)/g);
  for (const match of containerMinScaleMatches) {
    const val = Number(match[1]);
    if (val !== 0) {
      violations.push(`Expected min_scale = 0, found ${match[1]}`);
    }
  }

  const containerMaxScaleMatches = content.matchAll(/max_scale\s*=\s*([^\s\n]+)/g);
  for (const match of containerMaxScaleMatches) {
    const val = Number(match[1]);
    if (val > 1) {
      violations.push(`Expected max_scale <= 1 in V1 scale-to-zero, found ${match[1]}`);
    }
  }

  // Match scaleway_sdb_sql_database min_cpu / max_cpu
  const dbMinCpuMatches = content.matchAll(/min_cpu\s*=\s*([^\s\n]+)/g);
  for (const match of dbMinCpuMatches) {
    const val = Number(match[1]);
    if (val !== 0) {
      violations.push(`Expected min_cpu = 0, found ${match[1]}`);
    }
  }

  return {
    valid: violations.length === 0,
    violations,
  };
}

if (process.argv[1] && process.argv[1].endsWith('check-runtime-scale-to-zero.mjs')) {
  const target =
    process.argv[2] ?? 'infrastructure/environments/email-production/email-delivery.tf';
  const result = await verifyRuntimeScaleToZero(target);
  if (!result.valid) {
    console.error('Scale-to-zero runtime verification failed:\n' + result.violations.join('\n'));
    process.exit(1);
  }
  console.log(`Scale-to-zero runtime verified: ${target} satisfies zero-idle contract.`);
}
