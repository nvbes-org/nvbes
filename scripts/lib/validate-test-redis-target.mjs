import { isIP } from 'node:net';
import { pathToFileURL } from 'node:url';

export function validateTestRedisTarget(environment) {
  const raw = environment.NVBES_REDIS_URL;
  if (
    typeof raw !== 'string' ||
    raw !== raw.trim() ||
    raw.includes('\\') ||
    /[\u0000-\u0020\u007f]/u.test(raw)
  ) {
    throw new Error('NVBES_REDIS_URL must be an unambiguous Redis URL');
  }
  const syntax =
    /^(?:redis|rediss):\/\/(?:[^@/?#]*@)?(?:\[[0-9a-f:.]+\]|[^:/?#]+):([1-9][0-9]{0,4})\/(0|[1-9][0-9]{0,2})$/iu.exec(
      raw,
    );
  if (!syntax) {
    throw new Error('NVBES_REDIS_URL must contain a canonical explicit port and numeric database');
  }

  let target;
  try {
    target = new URL(raw);
  } catch {
    throw new Error('NVBES_REDIS_URL must be a valid Redis URL');
  }
  if (!['redis:', 'rediss:'].includes(target.protocol)) {
    throw new Error('NVBES_REDIS_URL must use redis or rediss');
  }
  if (target.username && target.username !== 'default') {
    throw new Error('NVBES_REDIS_URL must not contain an unexpected ACL username');
  }

  const hostname = target.hostname.replace(/^\[|\]$/gu, '').toLowerCase();
  const loopback =
    hostname === 'localhost' ||
    (isIP(hostname) === 4 && hostname.split('.')[0] === '127') ||
    (isIP(hostname) === 6 && hostname === '::1');
  if (!loopback) {
    throw new Error(`Account tests require loopback Redis; refusing ${hostname}`);
  }
  if (
    target.port !== syntax[1] ||
    !/^[1-9][0-9]{0,4}$/u.test(target.port) ||
    Number(target.port) > 65_535
  ) {
    throw new Error('NVBES_REDIS_URL must contain an explicit canonical port');
  }
  if (target.pathname !== `/${syntax[2]}` || !/^\/(?:0|[1-9][0-9]{0,2})$/u.test(target.pathname)) {
    throw new Error('NVBES_REDIS_URL must select an explicit numeric database');
  }
  if (target.search || target.hash) {
    throw new Error('NVBES_REDIS_URL must not contain query or fragment data');
  }
  return {
    database: Number(target.pathname.slice(1)),
    hostname,
    port: Number(target.port),
  };
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    validateTestRedisTarget(process.env);
  } catch (error) {
    process.stderr.write(
      `${error instanceof Error ? error.message : 'invalid Account test Redis target'}\n`,
    );
    process.exitCode = 2;
  }
}
