import { afterEach, expect, it, vi } from 'vite-plus/test';
import type { WorkerPowMessage } from './worker.pow';

afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

async function worker(hashes: number[][]) {
  vi.resetModules();
  const postMessage = vi.fn();
  const runtime = {
    postMessage,
    onmessage: null as ((event: MessageEvent<WorkerPowMessage>) => Promise<void>) | null,
  };
  const digest = vi.fn(async () => {
    const hash = hashes.shift();
    if (!hash) throw new Error('Unexpected additional hash');
    return new Uint8Array(hash).buffer;
  });
  vi.stubGlobal('self', runtime);
  vi.stubGlobal('crypto', { subtle: { digest } });
  await import('./worker.pow');
  return {
    digest,
    postMessage,
    run: (data: WorkerPowMessage) => runtime.onmessage?.(new MessageEvent('message', { data })),
  };
}

it.each([
  [0, [128]],
  [1, [64]],
  [7, [1]],
  [8, [0, 128]],
  [9, [0, 64]],
  [16, [0, 0]],
] as const)('accepts exactly %s leading zero bits', async (difficulty, bytes) => {
  const { run, digest, postMessage } = await worker([[...bytes]]);
  vi.spyOn(performance, 'now').mockReturnValueOnce(0).mockReturnValueOnce(1).mockReturnValue(10);
  await run({ nonce: 'test-nonce', difficulty });
  expect(digest).toHaveBeenCalledExactlyOnceWith(
    'SHA-256',
    new TextEncoder().encode('test-nonce:0'),
  );
  expect(postMessage).toHaveBeenCalledExactlyOnceWith({
    type: 'complete',
    solution: 0,
    hashesComputed: 1,
    hashesPerSecond: 100,
    estimatedDutyCycle: 0,
  });
});

it('increments candidates and reports batch progress while yielding between batches', async () => {
  vi.useFakeTimers();
  const { run, postMessage, digest } = await worker([[128], [64], [0]]);
  vi.spyOn(performance, 'now')
    .mockReturnValueOnce(0)
    .mockReturnValueOnce(0)
    .mockReturnValueOnce(300)
    .mockReturnValueOnce(315)
    .mockReturnValue(400);
  const pending = run({ nonce: 'batch', difficulty: 8, batchSize: 2, yieldDelayMs: 15 });
  await vi.advanceTimersByTimeAsync(0);
  expect(postMessage).toHaveBeenCalledExactlyOnceWith({
    type: 'progress',
    hashesComputed: 2,
    hashesPerSecond: 7,
    estimatedDutyCycle: 0.95,
  });
  expect(digest).toHaveBeenCalledTimes(2);
  await vi.advanceTimersByTimeAsync(14);
  expect(digest).toHaveBeenCalledTimes(2);
  await vi.advanceTimersByTimeAsync(1);
  await pending;
  expect(postMessage).toHaveBeenLastCalledWith({
    type: 'complete',
    solution: 2,
    hashesComputed: 3,
    hashesPerSecond: 8,
    estimatedDutyCycle: 0,
  });
  expect(vi.getTimerCount()).toBe(0);
});

it('does not emit progress before 300ms and allows a zero-delay batch', async () => {
  vi.useFakeTimers();
  const { run, postMessage } = await worker([[128], [0]]);
  vi.spyOn(performance, 'now')
    .mockReturnValueOnce(0)
    .mockReturnValueOnce(0)
    .mockReturnValueOnce(299)
    .mockReturnValueOnce(299)
    .mockReturnValue(300);
  await run({ nonce: 'batch', difficulty: 8, batchSize: 1, yieldDelayMs: 0 });
  expect(postMessage).toHaveBeenCalledExactlyOnceWith({
    type: 'complete',
    solution: 1,
    hashesComputed: 2,
    hashesPerSecond: 7,
    estimatedDutyCycle: 0,
  });
  expect(vi.getTimerCount()).toBe(0);
});
