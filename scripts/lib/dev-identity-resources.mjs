import { randomBytes, randomUUID } from 'node:crypto';
import {
  closeSync,
  linkSync,
  lstatSync,
  mkdirSync,
  openSync,
  readFileSync,
  unlinkSync,
  writeFileSync,
} from 'node:fs';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

export function ensureBillingResourceSecret(directory) {
  mkdirSync(directory, { recursive: true, mode: 0o700 });
  const target = join(directory, 'identity-billing-resource.secret');
  const temporary = join(directory, `.resource-${randomUUID()}.tmp`);
  const descriptor = openSync(temporary, 'wx', 0o600);
  try {
    try {
      writeFileSync(descriptor, randomBytes(32).toString('base64url'));
    } finally {
      closeSync(descriptor);
    }
    try {
      // Publish only a fully written file. Concurrent starters retain the first key.
      linkSync(temporary, target);
    } catch (error) {
      if (error.code !== 'EEXIST') throw error;
    }
  } finally {
    unlinkSync(temporary);
  }
  const metadata = lstatSync(target);
  if (!metadata.isFile() || (metadata.mode & 0o077) !== 0)
    throw new Error('Invalid local resource secret file');
  const secret = readFileSync(target, 'utf8');
  const bytes = Buffer.from(secret, 'base64url');
  if (bytes.length !== 32 || bytes.toString('base64url') !== secret) {
    throw new Error('Invalid local resource secret');
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  if (!process.argv[2]) throw new Error('Local key directory is required');
  ensureBillingResourceSecret(process.argv[2]);
}
