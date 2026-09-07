import { spawnSync } from 'node:child_process';
import { appendFileSync } from 'node:fs';
import { isTrustedPush } from './nx-cache-manager.core.mjs';

const env = { ...process.env };
const trusted = isTrustedPush(env.GITHUB_EVENT_NAME, env.GITHUB_REF);
const configured = env.AWS_ACCESS_KEY_ID && env.AWS_SECRET_ACCESS_KEY && env.SCW_CI_CACHE_BUCKET;
if (configured) {
  env.SCCACHE_BUCKET = env.SCW_CI_CACHE_BUCKET;
  env.SCCACHE_ENDPOINT = env.SCW_CI_CACHE_S3_ENDPOINT;
  env.SCCACHE_REGION = env.SCW_CI_CACHE_REGION;
  env.SCCACHE_S3_USE_SSL = 'true';
  env.SCCACHE_S3_ENABLE_VIRTUAL_HOST_STYLE = 'true';
  env.SCCACHE_S3_KEY_PREFIX = `trusted/rust/${env.RUNNER_OS}-${env.RUNNER_ARCH}/rust-1.91.1`;
  env.SCCACHE_S3_RW_MODE = trusted ? 'READ_WRITE' : 'READ_ONLY';
} else {
  // Forks and Dependabot use the ephemeral disk; no remote publication.
  for (const key of Object.keys(env))
    if (
      key.startsWith('SCCACHE_S3_') ||
      [
        'SCCACHE_BUCKET',
        'SCCACHE_ENDPOINT',
        'SCCACHE_REGION',
        'AWS_ACCESS_KEY_ID',
        'AWS_SECRET_ACCESS_KEY',
      ].includes(key)
    )
      delete env[key];
}
const result = spawnSync('sccache', ['--start-server'], { env, stdio: 'inherit' });
if (result.status !== 0) throw new Error('sccache startup failed');
appendFileSync(
  process.env.GITHUB_ENV,
  'RUSTC_WRAPPER=sccache\nCARGO_INCREMENTAL=0\nRUSTFLAGS=-C debuginfo=0\n',
);
