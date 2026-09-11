import { execFileSync } from 'node:child_process';
import { readlinkSync } from 'node:fs';
import path from 'node:path';

export function assertDescriptorConfined(descriptor, root, label) {
  const location = descriptorPath(descriptor);
  assert(
    location.startsWith(`${root}${path.sep}`) && !location.endsWith(' (deleted)'),
    `${label} escapes its trusted directory`,
  );
}

function descriptorPath(descriptor) {
  if (process.platform === 'linux') {
    return readlinkSync(`/proc/self/fd/${descriptor}`);
  }
  if (process.platform === 'darwin') {
    const entries = execFileSync(
      'lsof',
      ['-Fn', '-a', '-p', String(process.pid), '-d', String(descriptor)],
      { encoding: 'utf8', maxBuffer: 64 * 1024, windowsHide: true },
    )
      .split('\n')
      .filter((entry) => entry.startsWith('n'))
      .map((entry) => entry.slice(1));
    assert(entries.length === 1 && path.isAbsolute(entries[0]), 'Cannot resolve opened file');
    return entries[0];
  }
  throw new Error(`Secure descriptor paths are unavailable on ${process.platform}`);
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}
