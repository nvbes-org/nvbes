import { afterEach, beforeEach, expect, it, vi } from 'vite-plus/test';
import { getSwReady, sendToSw } from './service-worker.messaging';

class Port {
  onmessage: ((event: { data: unknown }) => void) | null = null;
  onmessageerror: (() => void) | null = null;
  close = vi.fn();
}
let channel: { port1: Port; port2: Port };
let postMessage: ReturnType<typeof vi.fn>;
beforeEach(() => {
  vi.useFakeTimers();
  channel = { port1: new Port(), port2: new Port() };
  vi.stubGlobal(
    'MessageChannel',
    class {
      port1 = channel.port1;
      port2 = channel.port2;
    },
  );
  vi.stubGlobal('crypto', { randomUUID: () => 'correlation-id' });
  postMessage = vi.fn();
  vi.stubGlobal('navigator', {
    serviceWorker: { ready: Promise.resolve({ active: { postMessage } }) },
  });
});
afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

function expectClosed() {
  expect(channel.port1.close).toHaveBeenCalledTimes(1);
  expect(channel.port2.close).toHaveBeenCalledTimes(1);
  expect(channel.port1.onmessage).toBeNull();
  expect(channel.port1.onmessageerror).toBeNull();
  expect(vi.getTimerCount()).toBe(0);
}
it('uses an already active registration without requiring another registration call', async () => {
  expect(await getSwReady()).toMatchObject({ active: { postMessage } });
  expect(vi.getTimerCount()).toBe(0);
});
it('bounds readiness at exactly 5s', async () => {
  vi.stubGlobal('navigator', { serviceWorker: { ready: new Promise(() => {}) } });
  const failure = expect(getSwReady()).rejects.toThrow('readiness timed out');
  await vi.advanceTimersByTimeAsync(4999);
  expect(vi.getTimerCount()).toBe(1);
  await vi.advanceTimersByTimeAsync(1);
  await failure;
  expect(postMessage).not.toHaveBeenCalled();
});
it('clears the readiness timer after rejection', async () => {
  vi.stubGlobal('navigator', { serviceWorker: { ready: Promise.reject(new Error('denied')) } });
  await expect(getSwReady()).rejects.toThrow('denied');
  expect(vi.getTimerCount()).toBe(0);
});
it.each([{}, { serviceWorker: { ready: Promise.resolve({ active: null }) } }])(
  'rejects missing active workers without opening a channel',
  async (navigator) => {
    vi.stubGlobal('navigator', navigator);
    await expect(sendToSw('READ')).rejects.toThrow('not active');
    expect(postMessage).not.toHaveBeenCalled();
    expect(vi.getTimerCount()).toBe(0);
  },
);
it('protects protocol fields and ignores replies for another request', async () => {
  const result = sendToSw('READ', { id: 'injected', type: 'WRITE', key: 'public' });
  await vi.advanceTimersByTimeAsync(0);
  expect(postMessage).toHaveBeenCalledExactlyOnceWith(
    { id: 'correlation-id', type: 'READ', key: 'public' },
    [channel.port2],
  );
  channel.port1.onmessage?.({ data: { id: 'other', result: 'wrong' } });
  expect(channel.port1.close).not.toHaveBeenCalled();
  channel.port1.onmessage?.({ data: { id: 'correlation-id', result: { value: 7 } } });
  expect(await result).toEqual({ value: 7 });
  expectClosed();
});
it.each(['remote', 'decode', 'post'])('releases every resource on %s failure', async (reason) => {
  if (reason === 'post')
    postMessage.mockImplementation(() => {
      throw new Error('post failed');
    });
  const failure = expect(sendToSw('READ')).rejects.toThrow(
    reason === 'decode' ? 'decoded' : `${reason} failed`,
  );
  await vi.advanceTimersByTimeAsync(0);
  if (reason === 'remote')
    channel.port1.onmessage?.({ data: { id: 'correlation-id', error: 'remote failed' } });
  if (reason === 'decode') channel.port1.onmessageerror?.();
  await failure;
  expectClosed();
});
it('times out unanswered requests without retrying them', async () => {
  const failure = expect(sendToSw('WRITE')).rejects.toThrow('response timed out');
  await vi.advanceTimersByTimeAsync(4999);
  expect(channel.port1.close).not.toHaveBeenCalled();
  await vi.advanceTimersByTimeAsync(1);
  await failure;
  expect(postMessage).toHaveBeenCalledTimes(1);
  expectClosed();
});
