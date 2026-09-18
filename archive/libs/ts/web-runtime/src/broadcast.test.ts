import { afterEach, expect, it, vi } from 'vite-plus/test';

afterEach(() => vi.unstubAllGlobals());

it('shares one channel, dispatches messages and respects unsubscription', async () => {
  vi.resetModules();
  const postMessage = vi.fn();
  const instances: Channel[] = [];
  class Channel {
    onmessage: ((event: MessageEvent) => void) | null = null;
    postMessage = postMessage;
    constructor(readonly name: string) {
      instances.push(this);
    }
  }
  vi.stubGlobal('BroadcastChannel', Channel);
  const api = await import('./broadcast');
  const first = vi.fn();
  const second = vi.fn();
  const unsubscribe = api.onBroadcast(first);
  api.onBroadcast(second);
  api.broadcastLogout('expired');
  api.broadcastWorkspaceChange('team-2');
  api.broadcastTokenRefreshed();
  expect(instances).toHaveLength(1);
  expect(instances[0]?.name).toBe('nvbes');
  expect(postMessage.mock.calls).toEqual([
    [{ type: 'logout', reason: 'expired' }],
    [{ type: 'workspace_change', workspaceId: 'team-2' }],
    [{ type: 'token_refreshed' }],
  ]);
  const message = { type: 'token_refreshed' };
  instances[0]?.onmessage?.(new MessageEvent('message', { data: message }));
  expect(first).toHaveBeenCalledExactlyOnceWith(message);
  expect(second).toHaveBeenCalledExactlyOnceWith(message);
  unsubscribe();
  instances[0]?.onmessage?.(new MessageEvent('message', { data: message }));
  expect(first).toHaveBeenCalledTimes(1);
  expect(second).toHaveBeenCalledTimes(2);
});
it('tolerates missing cross-tab messaging support', async () => {
  vi.resetModules();
  vi.stubGlobal('BroadcastChannel', undefined);
  const api = await import('./broadcast');
  const listener = vi.fn();
  const unsubscribe = api.onBroadcast(listener);
  expect(() => api.broadcastLogout()).not.toThrow();
  unsubscribe();
  expect(listener).not.toHaveBeenCalled();
});
