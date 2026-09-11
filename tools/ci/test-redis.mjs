import { appendFileSync } from 'node:fs';
import { startService, stopService } from './test-service.mjs';

if (process.argv[2] === 'stop') stopService('redis');
else {
  const { host, port } = await startService('redis');
  appendFileSync(process.env.GITHUB_ENV, `NVBES_REDIS_URL=redis://${host}:${port}/0\n`);
}
