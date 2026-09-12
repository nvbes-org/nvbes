import { afterEach, beforeEach, expect, it, vi } from 'vite-plus/test';

beforeEach(() => {
  vi.resetModules();
  vi.stubEnv('DEV', false);
  vi.stubGlobal('window', new EventTarget());
  vi.stubGlobal('document', { readyState: 'complete' });
  vi.stubGlobal('location', new URL('https://app.test/'));
  vi.stubGlobal('__nvbesTrustedTypesPolicy', null);
});
afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  vi.unstubAllEnvs();
});

it.each(['complete', 'loading'])(
  'registers the exact approved script after document state %s',
  async (state) => {
    const registration = Object.assign(new EventTarget(), {
      installing: null as (EventTarget & { state: string }) | null,
    });
    const register = vi.fn().mockResolvedValue(registration);
    vi.stubGlobal('navigator', { serviceWorker: { register, controller: {} } });
    vi.stubGlobal('document', { readyState: state });
    const runtime = await import('./service-worker');
    runtime.registerServiceWorker();
    if (state === 'loading') {
      expect(register).not.toHaveBeenCalled();
      window.dispatchEvent(new Event('load'));
    }
    await vi.waitFor(() => expect(register).toHaveBeenCalledExactlyOnceWith('/sw.js'));
    window.dispatchEvent(new Event('load'));
    expect(register).toHaveBeenCalledTimes(1);
    registration.dispatchEvent(new Event('updatefound'));
    const installing = Object.assign(new EventTarget(), { state: 'installing' });
    registration.installing = installing;
    registration.dispatchEvent(new Event('updatefound'));
    installing.dispatchEvent(new Event('statechange'));
    installing.state = 'installed';
    installing.dispatchEvent(new Event('statechange'));
    expect(register).toHaveBeenCalledTimes(1);
  },
);

it('does not register in development or on unsupported browsers', async () => {
  const register = vi.fn();
  vi.stubGlobal('navigator', { serviceWorker: { register } });
  const runtime = await import('./service-worker');
  vi.stubEnv('DEV', true);
  runtime.registerServiceWorker();
  expect(register).not.toHaveBeenCalled();
  vi.stubEnv('DEV', false);
  vi.stubGlobal('navigator', {});
  runtime.registerServiceWorker();
  expect(register).not.toHaveBeenCalled();
});

it('contains registration errors without announcing readiness', async () => {
  const error = new Error('registration denied');
  const report = vi.spyOn(console, 'error').mockImplementation(() => {});
  vi.stubGlobal('navigator', { serviceWorker: { register: vi.fn().mockRejectedValue(error) } });
  (await import('./service-worker')).registerServiceWorker();
  await vi.waitFor(() =>
    expect(report).toHaveBeenCalledExactlyOnceWith(
      '[SW] Service worker registration failed:',
      error,
    ),
  );
});
