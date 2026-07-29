import assert from 'node:assert/strict';
import test from 'node:test';

import { validateTestRedisTarget } from '../../scripts/lib/validate-test-redis-target.mjs';

test('Redis guard accepts explicit loopback port and database targets', () => {
  for (const target of [
    'redis://localhost:6379/0',
    'redis://:redis_dev@127.42.0.1:16379/15',
    'rediss://default:secret@[::1]:6380/1',
  ]) {
    assert.doesNotThrow(() => validateTestRedisTarget({ NVBES_REDIS_URL: target }));
  }
});

test('Redis guard rejects remote, ambiguous and underspecified targets', () => {
  for (const target of [
    'redis://127.0.0.1:6379@remote.example:6379/0',
    'redis://remote.example:6379/15',
    'redis://localhost/15',
    'redis://localhost:6379',
    'redis://localhost:06379/15',
    'redis://localhost:6379/not-a-db',
    'redis://unexpected:secret@localhost:6379/15',
    'https://localhost:6379/15',
    'redis://localhost:6379/15?tls=false',
  ]) {
    assert.throws(() => validateTestRedisTarget({ NVBES_REDIS_URL: target }));
  }
});
