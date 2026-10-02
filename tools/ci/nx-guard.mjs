#!/usr/bin/env node
import { existsSync, readdirSync, statSync, unlinkSync, rmSync } from 'node:fs';
import { join } from 'node:path';
import { execSync } from 'node:child_process';

const NX_TMP_DIR = join(process.cwd(), '.nx');
const NX_WORKSPACE_DATA = join(NX_TMP_DIR, 'workspace-data');

function cleanStaleLocks() {
  if (!existsSync(NX_WORKSPACE_DATA)) return;

  try {
    const files = readdirSync(NX_WORKSPACE_DATA);
    const now = Date.now();
    const STALE_THRESHOLD_MS = 5 * 60 * 1000; // 5 minutes

    for (const file of files) {
      if (file.endsWith('-wal') || file.endsWith('-shm') || file.endsWith('.lock')) {
        const filePath = join(NX_WORKSPACE_DATA, file);
        try {
          const stats = statSync(filePath);
          if (now - stats.mtimeMs > STALE_THRESHOLD_MS) {
            unlinkSync(filePath);
            console.log(`[nx-guard] Removed stale lock file: ${file}`);
          }
        } catch {}
      }
    }
  } catch (err) {
    // Non-fatal
  }
}

cleanStaleLocks();
