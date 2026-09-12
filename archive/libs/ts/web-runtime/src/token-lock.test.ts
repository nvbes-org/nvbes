import { afterEach, beforeEach, expect, it, vi } from 'vite-plus/test';
import { acquireTokenRefreshLock, withTokenRefreshLock } from './token-lock';

beforeEach(() => vi.useFakeTimers());
afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

it.each([undefined, {}])(
  'refreshes and notifies without the Web Locks API (%j)',
  async (navigator) => {
    vi.stubGlobal('navigator', navigator);
    const events: string[] = [];
    await expect(acquireTokenRefreshLock()).resolves.toBe(true);
    await withTokenRefreshLock(
      async () => {
        events.push('refresh');
      },
      () => {
        events.push('ready');
      },
    );
    expect(events).toEqual(['refresh', 'ready']);
    expect(vi.getTimerCount()).toBe(0);
  },
);
it('holds one lock for the complete refresh and notifies only after success', async () => {
  const events: string[] = [];
  const request = vi.fn(
    async (name: string, options: LockOptions, operation: () => Promise<void>) => {
      expect(name).toBe('nvbes-token-refresh');
      expect(options.signal).toBeInstanceOf(AbortSignal);
      events.push('acquired');
      await operation();
      events.push('released');
    },
  );
  vi.stubGlobal('navigator', { locks: { request } });
  await withTokenRefreshLock(
    async () => {
      events.push('refresh');
      await vi.advanceTimersByTimeAsync(6000);
      expect(request.mock.calls[0]?.[1].signal?.aborted).toBe(false);
      events.push('refreshed');
    },
    () => {
      events.push('ready');
    },
  );
  expect(events).toEqual(['acquired', 'refresh', 'refreshed', 'ready', 'released']);
  expect(request).toHaveBeenCalledTimes(1);
  expect(vi.getTimerCount()).toBe(0);
});
it('does not swallow refresh failures or announce a token that was not renewed', async () => {
  vi.stubGlobal('navigator', {
    locks: {
      request: async (_name: string, _options: LockOptions, operation: () => Promise<void>) =>
        operation(),
    },
  });
  const ready = vi.fn();
  await expect(
    withTokenRefreshLock(async () => {
      throw new Error('refresh rejected');
    }, ready),
  ).rejects.toThrow('refresh rejected');
  expect(ready).not.toHaveBeenCalled();
  expect(vi.getTimerCount()).toBe(0);
});
it('rejects an acquisition failure and always clears its deadline', async () => {
  vi.stubGlobal('navigator', {
    locks: { request: vi.fn().mockRejectedValue(new Error('denied')) },
  });
  const refresh = vi.fn();
  const ready = vi.fn();
  await expect(withTokenRefreshLock(refresh, ready)).rejects.toThrow('denied');
  await expect(acquireTokenRefreshLock()).resolves.toBe(false);
  expect(refresh).not.toHaveBeenCalled();
  expect(ready).not.toHaveBeenCalled();
  expect(vi.getTimerCount()).toBe(0);
});
it('aborts a queued acquisition at exactly five seconds without unbounded retry', async () => {
  const request = vi.fn(
    (_name: string, options: LockOptions) =>
      new Promise<void>((_resolve, reject) => {
        options.signal?.addEventListener('abort', () => reject(options.signal?.reason));
      }),
  );
  vi.stubGlobal('navigator', { locks: { request } });
  const refresh = vi.fn();
  const ready = vi.fn();
  const result = expect(withTokenRefreshLock(refresh, ready)).rejects.toThrow();
  await vi.advanceTimersByTimeAsync(4999);
  expect(request.mock.calls[0]?.[1].signal?.aborted).toBe(false);
  await vi.advanceTimersByTimeAsync(1);
  await result;
  expect(request).toHaveBeenCalledTimes(1);
  expect(refresh).not.toHaveBeenCalled();
  expect(ready).not.toHaveBeenCalled();
  expect(vi.getTimerCount()).toBe(0);
});
it('makes its compatibility probe release the lock and clear its timer', async () => {
  const request = vi.fn(
    async (_name: string, _options: LockOptions, operation: () => Promise<void>) => operation(),
  );
  vi.stubGlobal('navigator', { locks: { request } });
  await expect(acquireTokenRefreshLock()).resolves.toBe(true);
  expect(request).toHaveBeenCalledTimes(1);
  expect(vi.getTimerCount()).toBe(0);
});

it('keeps concurrent refreshes serialized until the previous operation completes', async () => {
  let queue = Promise.resolve();
  const request = vi.fn((_name: string, _options: LockOptions, operation: () => Promise<void>) => {
    const next = queue.then(operation);
    queue = next.catch(() => undefined);
    return next;
  });
  vi.stubGlobal('navigator', { locks: { request } });
  let finishFirst: (() => void) | undefined;
  const firstDone = new Promise<void>((resolve) => {
    finishFirst = resolve;
  });
  const events: string[] = [];
  const first = withTokenRefreshLock(
    async () => {
      events.push('first-start');
      await firstDone;
      events.push('first-end');
    },
    () => {
      events.push('first-ready');
    },
  );
  const second = withTokenRefreshLock(
    async () => {
      events.push('second');
    },
    () => {
      events.push('second-ready');
    },
  );
  await vi.advanceTimersByTimeAsync(0);
  expect(events).toEqual(['first-start']);
  finishFirst?.();
  await Promise.all([first, second]);
  expect(events).toEqual(['first-start', 'first-end', 'first-ready', 'second', 'second-ready']);
  expect(request).toHaveBeenCalledTimes(2);
  expect(vi.getTimerCount()).toBe(0);
});
