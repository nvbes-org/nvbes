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
// Only constant messages may reach logs; never interpolate the environment.
if (env.SCCACHE_GHA_ENABLED) console.log('Rust compilation cache: GitHub PR-scoped cache');
else if (env.SCCACHE_BUCKET) console.log('Rust compilation cache: protected Scaleway cache');
else console.log('Rust compilation cache: ephemeral local cache');
appendFileSync(
  process.env.GITHUB_ENV,
  `SCCACHE_SERVER_UDS=${env.SCCACHE_SERVER_UDS}\nRUSTC_WRAPPER=sccache\nCARGO_INCREMENTAL=0\nRUSTFLAGS=-C debuginfo=0\n`,
);
