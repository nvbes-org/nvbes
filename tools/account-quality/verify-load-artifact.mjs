import { readFile } from 'node:fs/promises';
import process from 'node:process';

const artifactPath = process.argv[2];
if (!artifactPath) {
  throw new Error('Usage: node verify-load-artifact.mjs <summary-path>');
}

const source = await readFile(artifactPath, 'utf8');
const summary = JSON.parse(source);

if (!isRecord(summary.metrics) || !isRecord(summary.root_group)) {
  throw new Error('k6 summary is missing required metrics or group evidence.');
}
if (summary.setup_data !== undefined && summary.setup_data !== null) {
  throw new Error('k6 setup_data must be empty so credentials can never enter evidence.');
}

const secret = process.env.ACCOUNT_TEST_COOKIE;
if (secret && source.includes(secret)) {
  throw new Error('k6 summary contains the session credential.');
}

rejectSensitiveKeys(summary);
process.stdout.write('Validated redacted k6 evidence.\n');

function rejectSensitiveKeys(value) {
  if (Array.isArray(value)) {
    for (const item of value) {
      rejectSensitiveKeys(item);
    }
    return;
  }
  if (!isRecord(value)) {
    return;
  }
  for (const [key, nested] of Object.entries(value)) {
    if (/^(authorization|cookie|password|secret|set-cookie)$/iu.test(key)) {
      throw new Error('k6 summary contains a forbidden credential-bearing field.');
    }
    rejectSensitiveKeys(nested);
  }
}

function isRecord(value) {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}
