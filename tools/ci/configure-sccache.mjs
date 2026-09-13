import { spawnSync } from 'node:child_process';
import { appendFileSync, mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { compilationCacheEnvironment } from './configure-sccache.core.mjs';

const env = compilationCacheEnvironment(process.env);
// Own a fresh daemon without stopping another job's server on a reused runner.
env.SCCACHE_SERVER_UDS = path.join(mkdtempSync(path.join(tmpdir(), 'sc-')), 's');
const result = spawnSync('sccache', ['--start-server'], { env, stdio: 'inherit' });
if (result.status !== 0) throw new Error('sccache startup failed');
const backend = env.SCCACHE_GHA_ENABLED
  ? 'GitHub PR-scoped cache'
  : env.SCCACHE_BUCKET
    ? `Scaleway ${env.SCCACHE_S3_RW_MODE}`
    : 'ephemeral local cache';
console.log(`Rust compilation cache: ${backend}`);
appendFileSync(
  process.env.GITHUB_ENV,
  `SCCACHE_SERVER_UDS=${env.SCCACHE_SERVER_UDS}\nRUSTC_WRAPPER=sccache\nCARGO_INCREMENTAL=0\nRUSTFLAGS=-C debuginfo=0\n`,
);
