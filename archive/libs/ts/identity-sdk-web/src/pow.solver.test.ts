import { afterEach, beforeEach, expect, it, vi } from 'vite-plus/test';
import { DeviceMonitor } from './device-monitor';
import { fetchPowChallenge, solvePowChallenge } from './pow';

const state = {
  cpuCores: 2,
  isLowPowerMode: false,
  targetCpuLoad: 0.2,
  recommendedBatchSize: 2,
  recommendedYieldDelayMs: 12,
};
beforeEach(() => {
  vi.spyOn(DeviceMonitor.prototype, 'getPerformanceState').mockReturnValue(state);
  vi.stubGlobal('navigator', { hardwareConcurrency: 2 });
  vi.stubGlobal('Worker', undefined);
});
afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  vi.useRealTimers();
});

it('forwards worker progress and completion, preserving device load configuration', async () => {
  const terminate = vi.fn();
  class WorkerStub {
    onmessage?: (event: { data: Record<string, unknown> }) => void;
    onerror?: (error: Error) => void;
    terminate = terminate;
    constructor(url: URL, options: WorkerOptions) {
      expect(url.pathname).toContain('worker.pow.ts');
      expect(options).toEqual({ type: 'module' });
    }
    postMessage(message: unknown) {
      expect(message).toEqual({ nonce: 'nonce', difficulty: 8, batchSize: 2, yieldDelayMs: 12 });
      this.onmessage?.({ data: { type: 'progress', hashesComputed: 2, hashesPerSecond: 20 } });
      this.onmessage?.({
        data: {
          type: 'complete',
          solution: 2,
          hashesComputed: 3,
          hashesPerSecond: 30,
          estimatedDutyCycle: 0.1,
        },
      });
    }
  }
  vi.stubGlobal('Worker', WorkerStub);
  const progress = vi.fn();
  expect(await solvePowChallenge('nonce', 8, { onProgress: progress, targetCpuLoad: 0.2 })).toBe(2);
  expect(progress.mock.calls.map(([value]) => value)).toEqual([
    {
      solution: undefined,
      hashesComputed: 2,
      hashesPerSecond: 20,
      estimatedCpuDutyCycle: 0.2,
      deviceState: state,
    },
    {
      solution: 2,
      hashesComputed: 3,
      hashesPerSecond: 30,
      estimatedCpuDutyCycle: 0.1,
      deviceState: state,
    },
  ]);
  expect(terminate).toHaveBeenCalledTimes(1);
});

it('falls back after a worker error and terminates the failed worker', async () => {
  const terminate = vi.fn();
  class WorkerStub {
    onerror?: (error: Error) => void;
    terminate = terminate;
    postMessage() {
      this.onerror?.(new Error('worker unavailable'));
    }
  }
  vi.stubGlobal('Worker', WorkerStub);
  vi.stubGlobal('crypto', {
    subtle: { digest: vi.fn().mockResolvedValue(new Uint8Array([0]).buffer) },
  });
  expect(await solvePowChallenge('nonce', 8)).toBe(0);
  expect(terminate).toHaveBeenCalledTimes(1);
});

it.each([10, 25])(
  'yields between main-thread batches and reports exact progress (delay %s)',
  async (delay) => {
    vi.useFakeTimers();
    vi.spyOn(DeviceMonitor.prototype, 'getPerformanceState').mockReturnValue({
      ...state,
      recommendedYieldDelayMs: delay === 10 ? 1 : delay,
    });
    let clock = 0;
    vi.spyOn(performance, 'now').mockImplementation(() => clock);
    const inputs: string[] = [];
    vi.stubGlobal('crypto', {
      subtle: {
        digest: vi.fn(async (algorithm: string, bytes: Uint8Array) => {
          expect(algorithm).toBe('SHA-256');
          inputs.push(new TextDecoder().decode(bytes));
          clock += 200;
          return new Uint8Array(inputs.length === 3 ? [0, 0x40] : [0x80]).buffer;
        }),
      },
    });
    const progress = vi.fn();
    const result = solvePowChallenge('nonce', 9, { onProgress: progress });
    await vi.advanceTimersByTimeAsync(0);
    expect(inputs).toEqual(['nonce:0', 'nonce:1']);
    expect(progress).toHaveBeenCalledWith({
      hashesComputed: 2,
      hashesPerSecond: 5,
      estimatedCpuDutyCycle: 0.2,
      deviceState: { ...state, recommendedYieldDelayMs: delay === 10 ? 1 : delay },
    });
    await vi.advanceTimersByTimeAsync(delay - 1);
    expect(inputs).toHaveLength(2);
    await vi.advanceTimersByTimeAsync(1);
    expect(await result).toBe(2);
    expect(progress).toHaveBeenLastCalledWith(
      expect.objectContaining({
        solution: 2,
        hashesComputed: 3,
        hashesPerSecond: 5,
        estimatedCpuDutyCycle: 0,
      }),
    );
    expect(vi.getTimerCount()).toBe(0);
  },
);

it('fetches challenges with cookies and rejects HTTP failures', async () => {
  const fetcher = vi.fn().mockResolvedValue(Response.json({ nonce: 'n', difficulty: 8 }));
  vi.stubGlobal('fetch', fetcher);
  expect(await fetchPowChallenge('https://identity.test')).toEqual({ nonce: 'n', difficulty: 8 });
  expect(fetcher).toHaveBeenCalledExactlyOnceWith('https://identity.test/auth/challenge/pow', {
    credentials: 'include',
  });
  fetcher.mockResolvedValue(new Response(null, { status: 503 }));
  await expect(fetchPowChallenge('https://identity.test')).rejects.toThrow('503');
});
