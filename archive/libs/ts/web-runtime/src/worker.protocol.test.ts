// @vitest-environment happy-dom
import { afterEach, expect, it, vi } from 'vite-plus/test';
import { createWorker, defineWorker } from './worker';

afterEach(() => vi.unstubAllGlobals());

it('dispatches worker requests and serializes results and both error kinds', async () => {
  const runtime = {
    onmessage: null as ((event: MessageEvent) => Promise<void>) | null,
    postMessage: vi.fn(),
  };
  vi.stubGlobal('self', runtime);
  defineWorker({
    add: (a: number, b: number) => a + b,
    fail: () => {
      throw new Error('failed');
    },
    reject: () => Promise.reject('rejected'),
  });
  for (const [id, method, args, expected] of [
    [1, 'add', [2, 3], { id: 1, result: 5 }],
    [2, 'absent', [], { id: 2, error: 'Unknown method: absent' }],
    [3, 'fail', [], { id: 3, error: 'failed' }],
    [4, 'reject', [], { id: 4, error: 'rejected' }],
  ] as const) {
    await runtime.onmessage?.(new MessageEvent('message', { data: { id, method, args } }));
    expect(runtime.postMessage).toHaveBeenLastCalledWith(expected);
  }
});

it('creates one worker lazily and correlates concurrent replies even out of order', async () => {
  const worker = {
    postMessage: vi.fn(),
    onmessage: null as ((event: MessageEvent) => void) | null,
    onerror: null as ((event: ErrorEvent) => void) | null,
  };
  const factory = vi.fn(() => worker as unknown as Worker);
  const client = createWorker<{ add: (a: number, b: number) => number }>(factory);
  expect(factory).not.toHaveBeenCalled();
  const first = client.add(1, 2);
  const second = client.add(3, 4);
  expect(factory).toHaveBeenCalledTimes(1);
  expect(worker.postMessage.mock.calls).toEqual([
    [{ id: 1, method: 'add', args: [1, 2] }],
    [{ id: 2, method: 'add', args: [3, 4] }],
  ]);
  worker.onmessage?.(new MessageEvent('message', { data: { id: 999, result: 'ignored' } }));
  worker.onmessage?.(new MessageEvent('message', { data: { id: 2, result: 7 } }));
  worker.onmessage?.(new MessageEvent('message', { data: { id: 1, result: 3 } }));
  await expect(first).resolves.toBe(3);
  await expect(second).resolves.toBe(7);
  const failure = expect(client.add(0, 0)).rejects.toThrow('remote error');
  worker.onmessage?.(new MessageEvent('message', { data: { id: 3, error: 'remote error' } }));
  await failure;
});

it.each(['crashed', ''])('rejects all pending calls on worker failure %s', async (message) => {
  const worker = { postMessage: vi.fn(), onerror: null as ((event: ErrorEvent) => void) | null };
  const client = createWorker<{ work: () => string }>(() => worker as unknown as Worker);
  const failures = [
    expect(client.work()).rejects.toThrow(message || 'Worker error'),
    expect(client.work()).rejects.toThrow(message || 'Worker error'),
  ];
  worker.onerror?.(new ErrorEvent('error', { message }));
  await Promise.all(failures);
  worker.onerror?.(new ErrorEvent('error', { message }));
});
