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
const hasLld =
  (process.platform === 'linux' || env.RUNNER_OS === 'Linux') &&
  spawnSync('which', ['lld']).status === 0;
const rustflags = hasLld ? '-C debuginfo=0 -C link-arg=-fuse-ld=lld' : '-C debuginfo=0';
appendFileSync(
  process.env.GITHUB_ENV,
  `SCCACHE_SERVER_UDS=${env.SCCACHE_SERVER_UDS}\nRUSTC_WRAPPER=sccache\nCARGO_INCREMENTAL=0\nRUSTFLAGS=${rustflags}\n`,
);
