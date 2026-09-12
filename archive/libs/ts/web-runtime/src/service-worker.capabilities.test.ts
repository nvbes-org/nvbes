import { afterEach, expect, it, vi } from 'vite-plus/test';
import {
  clearAppBadge,
  getWebAppDisplayMode,
  getWebPushSupport,
  getSwReady,
  isInstalledWebApp,
  migrateServiceWorkers,
  onBackgroundFetchEvent,
  registerPeriodicSync,
  requestWebPushPermission,
  sendNetworkQualityToSw,
  setAppBadge,
  startBackgroundFetch,
  supportsAppBadging,
} from './service-worker';

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

it.each(['window-controls-overlay', 'fullscreen', 'standalone', 'minimal-ui', 'browser'] as const)(
  'detects %s display mode',
  (mode) => {
    vi.stubGlobal('window', {
      matchMedia: (query: string) => ({ matches: query === `(display-mode: ${mode})` }),
    });
    vi.stubGlobal('navigator', {});
    expect(getWebAppDisplayMode()).toBe(mode);
    expect(isInstalledWebApp()).toBe(mode !== 'browser');
  },
);
it('uses iOS standalone and otherwise defaults to the browser without matchMedia', () => {
  vi.stubGlobal('window', {});
  vi.stubGlobal('navigator', { standalone: true });
  expect(getWebAppDisplayMode()).toBe('standalone');
  vi.stubGlobal('navigator', {});
  expect(getWebAppDisplayMode()).toBe('browser');
});
it('prefers overlay over the other simultaneously matched modes', () => {
  vi.stubGlobal('window', { matchMedia: () => ({ matches: true }) });
  expect(getWebAppDisplayMode()).toBe('window-controls-overlay');
});
it('requires both badge methods and handles missing, successful and denied badges', async () => {
  vi.stubGlobal('navigator', {});
  expect(supportsAppBadging()).toBe(false);
  await expect(setAppBadge(3)).resolves.toBe(false);
  await expect(clearAppBadge()).resolves.toBe(false);
  const set = vi.fn().mockResolvedValue(undefined);
  const clear = vi.fn().mockResolvedValue(undefined);
  vi.stubGlobal('navigator', { setAppBadge: set });
  expect(supportsAppBadging()).toBe(false);
  vi.stubGlobal('navigator', { setAppBadge: set, clearAppBadge: clear });
  expect(supportsAppBadging()).toBe(true);
  await expect(setAppBadge(7)).resolves.toBe(true);
  expect(set).toHaveBeenCalledExactlyOnceWith(7);
  await expect(clearAppBadge()).resolves.toBe(true);
  expect(clear).toHaveBeenCalledExactlyOnceWith();
  set.mockRejectedValue(new Error('denied'));
  clear.mockRejectedValue(new Error('denied'));
  await expect(setAppBadge()).resolves.toBe(false);
  await expect(clearAppBadge()).resolves.toBe(false);
});
it('distinguishes each missing push prerequisite without requesting permission', async () => {
  const requestPermission = vi.fn();
  vi.stubGlobal('Notification', { permission: 'denied', requestPermission });
  vi.stubGlobal('window', {});
  vi.stubGlobal('navigator', {});
  expect(getWebPushSupport()).toEqual({
    supported: false,
    permission: 'unsupported',
    reason: 'missing-notifications',
  });
  await expect(requestWebPushPermission()).resolves.toBe('unsupported');
  vi.stubGlobal('window', { Notification: {} });
  expect(getWebPushSupport()).toEqual({
    supported: false,
    permission: 'denied',
    reason: 'missing-service-worker',
  });
  vi.stubGlobal('navigator', { serviceWorker: {} });
  expect(getWebPushSupport()).toEqual({
    supported: false,
    permission: 'denied',
    reason: 'missing-push-manager',
  });
  await expect(requestWebPushPermission()).resolves.toBe('denied');
  expect(requestPermission).not.toHaveBeenCalled();
});
it.each(['default', 'granted', 'denied'] as const)(
  'requests push permission only from %s',
  async (permission) => {
    const requestPermission = vi.fn().mockResolvedValue('granted');
    vi.stubGlobal('window', { Notification: {}, PushManager: {} });
    vi.stubGlobal('navigator', { serviceWorker: {} });
    vi.stubGlobal('Notification', { permission, requestPermission });
    expect(getWebPushSupport()).toEqual({ supported: true, permission });
    await expect(requestWebPushPermission()).resolves.toBe(
      permission === 'default' ? 'granted' : permission,
    );
    expect(requestPermission).toHaveBeenCalledTimes(permission === 'default' ? 1 : 0);
  },
);
it('reports unsupported background facilities without a service worker or registration capability', async () => {
  for (const navigator of [{}, { serviceWorker: { ready: Promise.resolve({}) } }]) {
    vi.stubGlobal('navigator', navigator);
    await expect(registerPeriodicSync('refresh')).resolves.toBe(false);
    await expect(startBackgroundFetch('job', ['/a'], 'Download')).resolves.toBe(false);
  }
  vi.stubGlobal('navigator', {});
  await expect(getSwReady()).resolves.toBeNull();
  await expect(migrateServiceWorkers()).resolves.toBe(0);
});
it('forwards background operations and reports permission failures', async () => {
  const register = vi.fn().mockResolvedValue(undefined);
  const fetch = vi.fn().mockResolvedValue(undefined);
  vi.stubGlobal('navigator', {
    serviceWorker: {
      ready: Promise.resolve({ periodicSync: { register }, backgroundFetch: { fetch } }),
    },
  });
  await expect(registerPeriodicSync('daily')).resolves.toBe(true);
  expect(register).toHaveBeenLastCalledWith('daily', { minInterval: 86400000 });
  await expect(registerPeriodicSync('hourly', 3600000)).resolves.toBe(true);
  expect(register).toHaveBeenLastCalledWith('hourly', { minInterval: 3600000 });
  await expect(startBackgroundFetch('export-1', ['/export'], 'Export')).resolves.toBe(true);
  expect(fetch).toHaveBeenCalledExactlyOnceWith('export-1', ['/export'], { title: 'Export' });
  register.mockRejectedValue(new Error('denied'));
  fetch.mockRejectedValue(new Error('denied'));
  await expect(registerPeriodicSync('daily')).resolves.toBe(false);
  await expect(startBackgroundFetch('export-1', [], 'Export')).resolves.toBe(false);
});
it('filters background messages and removes listeners on unsubscribe', () => {
  const serviceWorker = new EventTarget();
  vi.stubGlobal('navigator', { serviceWorker });
  const callback = vi.fn();
  const unsubscribe = onBackgroundFetchEvent(callback);
  for (const data of [null, {}, { type: 'sync' }, { type: 'backgroundfetchsuccess', id: 'one' }]) {
    serviceWorker.dispatchEvent(new MessageEvent('message', { data }));
  }
  expect(callback).toHaveBeenCalledExactlyOnceWith({ type: 'backgroundfetchsuccess', id: 'one' });
  unsubscribe();
  serviceWorker.dispatchEvent(
    new MessageEvent('message', { data: { type: 'backgroundfetchfail', id: 'two' } }),
  );
  expect(callback).toHaveBeenCalledTimes(1);
});
it('sends network quality only when an active worker is available', async () => {
  const quality = {
    effectiveType: '3g',
    isSlowConnection: true,
    saveData: true,
    downlink: 1,
    rtt: 400,
  };
  const postMessage = vi.fn();
  for (const registration of [{}, { active: { postMessage } }]) {
    vi.stubGlobal('navigator', { serviceWorker: { ready: Promise.resolve(registration) } });
    await sendNetworkQualityToSw(quality);
  }
  expect(postMessage).toHaveBeenCalledExactlyOnceWith({ type: 'NETWORK_QUALITY', ...quality });
  vi.stubGlobal('navigator', {});
  await expect(sendNetworkQualityToSw(quality)).resolves.toBeUndefined();
  vi.stubGlobal('navigator', {
    serviceWorker: {
      get ready() {
        throw new Error('unavailable');
      },
    },
  });
  await expect(sendNetworkQualityToSw(quality)).resolves.toBeUndefined();
});
it('retains current workers and unregisters obsolete registrations', async () => {
  const current = ['active', 'installing', 'waiting'].map((state) => ({
    [state]: { scriptURL: 'https://app.example.test/sw.js' },
    unregister: vi.fn(),
  }));
  const old = [
    { active: { scriptURL: '/old.js' }, unregister: vi.fn().mockResolvedValue(true) },
    { unregister: vi.fn().mockResolvedValue(true) },
  ];
  vi.stubGlobal('navigator', {
    serviceWorker: { getRegistrations: async () => [...current, ...old] },
  });
  await expect(migrateServiceWorkers()).resolves.toBe(2);
  for (const registration of current) expect(registration.unregister).not.toHaveBeenCalled();
  for (const registration of old) expect(registration.unregister).toHaveBeenCalledExactlyOnceWith();
});
