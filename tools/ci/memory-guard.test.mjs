import assert from 'node:assert/strict';
import test from 'node:test';
import {
  checkOomExit,
  getMemoryInfo,
  recommendedParallelism,
  startMemoryWatchdog,
  stopMemoryWatchdog,
} from './memory-guard.mjs';

test('getMemoryInfo returns valid memory metrics', () => {
  const mem = getMemoryInfo();
  assert.ok(typeof mem.totalMb === 'number' && mem.totalMb > 0);
  assert.ok(typeof mem.availableMb === 'number' && mem.availableMb >= 0);
  assert.ok(
    typeof mem.percentAvailable === 'number' &&
      mem.percentAvailable >= 0 &&
      mem.percentAvailable <= 100,
  );
});

test('recommendedParallelism respects bounds and constraints', () => {
  const p1 = recommendedParallelism({ maxParallel: 1, minParallel: 1, silent: true });
  assert.equal(p1, 1);

  const p2 = recommendedParallelism({
    maxParallel: 8,
    minParallel: 1,
    memoryPerWorkerMb: 500,
    silent: true,
  });
  assert.ok(p2 >= 1 && p2 <= 8);

  const pHighRequirement = recommendedParallelism({
    maxParallel: 4,
    minParallel: 1,
    memoryPerWorkerMb: 1000000,
    silent: true,
  });
  assert.equal(pHighRequirement, 1);
});

test('checkOomExit correctly recognizes exit code 137 and SIGKILL', () => {
  assert.equal(checkOomExit(137, null, 'test-task'), true);
  assert.equal(checkOomExit(null, 'SIGKILL', 'test-task'), true);
  assert.equal(checkOomExit(0, null, 'test-task'), false);
  assert.equal(checkOomExit(1, null, 'test-task'), false);
});

test('startMemoryWatchdog and stopMemoryWatchdog manage interval', async () => {
  startMemoryWatchdog({ intervalMs: 50, thresholdMb: 999999 });
  await new Promise((resolve) => setTimeout(resolve, 150));
  const stats = stopMemoryWatchdog();
  assert.ok(Number.isFinite(stats.minAvailableMbSeen));
});
