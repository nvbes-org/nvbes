import { afterEach, beforeEach, expect, it, vi } from 'vite-plus/test';
import { collectEnvironmentSignals } from './bot-guard.signals.environment';

beforeEach(() => {
  let timestamp = 1;
  vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => {
    timestamp += 16;
    callback(timestamp);
    return timestamp;
  });
  vi.stubGlobal('navigator', { userAgent: 'Firefox' });
  vi.stubGlobal('window', {});
  vi.stubGlobal('localStorage', { setItem: vi.fn(), getItem: () => '1', removeItem: vi.fn() });
});
afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

it('uses neutral missing-API defaults and measures stable frame intervals', async () => {
  expect(await collectEnvironmentSignals()).toEqual({
    speech_voices_count: -1,
    nav_touch_points: 0,
    nav_hw_concurrency: null,
    nav_device_memory: null,
    lang: '',
    tz: Intl.DateTimeFormat().resolvedOptions().timeZone,
    tz_offset: new Date().getTimezoneOffset(),
    perm_notifications: 'unavailable',
    raf_variance: 0,
    raf_mean: 16,
    battery_available: true,
    storage_ok: true,
  });
});
it.each(['granted', 'denied', 'prompt'])(
  'collects %s permissions, hardware and voices only after a gesture',
  async (state) => {
    const query = vi.fn().mockResolvedValue({ state });
    const voices = vi.fn(() => [{ name: 'Voice' }, { name: 'Voice2' }]);
    vi.stubGlobal('navigator', {
      userAgent: 'Chrome Mobile',
      language: 'fr',
      maxTouchPoints: 2,
      hardwareConcurrency: 8,
      deviceMemory: 4,
      getBattery: async () => ({}),
      permissions: { query },
    });
    vi.stubGlobal('window', { speechSynthesis: { getVoices: voices } });
    expect((await collectEnvironmentSignals()).speech_voices_count).toBe(-1);
    expect(voices).not.toHaveBeenCalled();
    expect(await collectEnvironmentSignals(true)).toMatchObject({
      speech_voices_count: 2,
      nav_touch_points: 2,
      nav_hw_concurrency: 8,
      nav_device_memory: 4,
      lang: 'fr',
      perm_notifications: state,
      battery_available: true,
    });
    expect(query).toHaveBeenCalledWith({ name: 'notifications' });
  },
);
it.each(['Chrome Mobile', 'Chrome Android', 'Chrome Desktop', 'Safari Mobile'])(
  'requires battery API only for Chrome mobile (%s)',
  async (userAgent) => {
    vi.stubGlobal('navigator', { userAgent });
    expect((await collectEnvironmentSignals()).battery_available).toBe(
      !['Chrome Mobile', 'Chrome Android'].includes(userAgent),
    );
  },
);
it('handles privacy-restricted APIs and unknown timezone without failing collection', async () => {
  vi.stubGlobal('navigator', {
    get userAgent() {
      throw new Error('private');
    },
    permissions: {
      query: async () => {
        throw new Error('denied');
      },
    },
  });
  vi.stubGlobal('window', {
    speechSynthesis: {
      getVoices() {
        throw new Error('denied');
      },
    },
  });
  vi.stubGlobal('localStorage', {
    setItem() {
      throw new Error('private');
    },
  });
  vi.spyOn(Intl, 'DateTimeFormat').mockImplementation(() => {
    throw new Error('unsupported');
  });
  expect(await collectEnvironmentSignals(true)).toMatchObject({
    tz: 'unknown',
    perm_notifications: 'unavailable',
    speech_voices_count: -1,
    battery_available: true,
    storage_ok: true,
  });
});
it('removes its temporary storage key and detects a failed storage roundtrip', async () => {
  const setItem = vi.fn();
  const removeItem = vi.fn();
  const getItem = vi.fn(() => null);
  vi.stubGlobal('localStorage', { setItem, getItem, removeItem });
  expect((await collectEnvironmentSignals(true)).storage_ok).toBe(false);
  const key: unknown = setItem.mock.calls[0]?.[0];
  expect(key).toMatch(/^_bg_/);
  expect(setItem).toHaveBeenCalledExactlyOnceWith(key, '1');
  expect(getItem).toHaveBeenCalledExactlyOnceWith(key);
  expect(removeItem).toHaveBeenCalledExactlyOnceWith(key);
});
it('computes nonzero animation frame variance from actual deltas', async () => {
  const timestamps = [10, 20, 40, 50, 70, 80, 100, 110, 130, 140];
  vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => {
    callback(timestamps.shift() ?? 0);
    return 1;
  });
  const result = await collectEnvironmentSignals();
  expect(result.raf_mean).toBeCloseTo(130 / 9);
  expect(result.raf_variance).toBeCloseTo(2000 / 81);
});
