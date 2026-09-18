import { afterEach, beforeEach, describe, expect, it, vi } from 'vite-plus/test';
import {
  createBrowserAnalyticsTransport,
  createBrowserAnalyticsTransportFromEnv,
} from './analytics.browser-transport';

const sdk = vi.hoisted(() => ({
  init: vi.fn(),
  captureException: vi.fn(),
  close: vi.fn(async () => true),
}));
vi.mock('@sentry/browser', () => sdk);
const productSdk = vi.hoisted(() => ({
  init: vi.fn(),
  capture: vi.fn(),
  stopSessionRecording: vi.fn(),
  opt_out_capturing: vi.fn(),
  reset: vi.fn(),
}));
vi.mock('posthog-js/dist/module.full.no-external', () => ({ default: productSdk }));
beforeEach(() => {
  vi.stubGlobal('window', {});
  vi.resetAllMocks();
  sdk.close.mockResolvedValue(true);
});
afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});
const options = {
  appName: 'v1-runtime',
  environment: 'production',
  sentryDsn: ' https://synthetic@sentry.test/1 ',
};

describe('error reporting consent lifecycle', () => {
  it('withdraws product analytics immediately even if error-reporting close fails', async () => {
    const transport = createBrowserAnalyticsTransport({ ...options, posthogKey: 'synthetic' });
    await transport.setProductAnalyticsEnabled?.(true);
    await transport.setErrorReportingEnabled?.(true);
    const failure = new Error('close failed');
    sdk.close.mockRejectedValueOnce(failure);
    const disabling = transport.disableCapture?.();
    const rejected = expect(disabling).rejects.toBe(failure);
    expect(productSdk.opt_out_capturing).toHaveBeenCalledTimes(1);
    await transport.trackProductEvent?.('marketing.page_viewed', {});
    expect(productSdk.capture).not.toHaveBeenCalled();
    await rejected;
  });

  it('serializes concurrent initialization and applies the privacy settings', async () => {
    const transport = createBrowserAnalyticsTransport(options);
    await Promise.all([
      transport.setErrorReportingEnabled?.(true),
      transport.setErrorReportingEnabled?.(true),
    ]);
    expect(sdk.init).toHaveBeenCalledExactlyOnceWith(
      expect.objectContaining({
        dsn: options.sentryDsn.trim(),
        environment: 'production',
        sendDefaultPii: false,
        beforeBreadcrumb: expect.any(Function),
        beforeSend: expect.any(Function),
        initialScope: { tags: { app_name: 'v1-runtime' } },
      }),
    );
    const error = new Error('synthetic');
    await transport.captureException?.(error, { status: 'failed' });
    expect(sdk.captureException).toHaveBeenCalledExactlyOnceWith(error, {
      extra: { status: 'failed' },
    });
    await transport.disableCapture?.();
    expect(sdk.close).toHaveBeenCalledExactlyOnceWith(2_000);
    await transport.captureException?.(error, {});
    expect(sdk.captureException).toHaveBeenCalledTimes(1);
  });

  it('does not initialize after withdrawal during SDK loading', async () => {
    const transport = createBrowserAnalyticsTransport(options);
    const enabling = transport.setErrorReportingEnabled?.(true);
    await Promise.resolve();
    await transport.setErrorReportingEnabled?.(false);
    await enabling;
    expect(sdk.init).not.toHaveBeenCalled();
    await transport.captureException?.(new Error('private'), {});
    expect(sdk.captureException).not.toHaveBeenCalled();
    await transport.setErrorReportingEnabled?.(true);
    expect(sdk.init).toHaveBeenCalledTimes(1);
  });

  it('waits for a pending close before reinitializing after a new consent', async () => {
    const transport = createBrowserAnalyticsTransport(options);
    await transport.setErrorReportingEnabled?.(true);
    let finishClose: (result: boolean) => void = () => {
      throw new Error('close not started');
    };
    sdk.close.mockImplementationOnce(
      () =>
        new Promise<boolean>((resolve) => {
          finishClose = resolve;
        }),
    );
    const disabling = transport.setErrorReportingEnabled?.(false);
    await Promise.resolve();
    const enabling = transport.setErrorReportingEnabled?.(true);
    await Promise.resolve();
    expect(sdk.init).toHaveBeenCalledTimes(1);
    await transport.captureException?.(new Error('during close'), {});
    expect(sdk.captureException).not.toHaveBeenCalled();
    finishClose(true);
    await Promise.all([disabling, enabling]);
    expect(sdk.init).toHaveBeenCalledTimes(2);
  });

  it('reports initialization failures and allows an explicit later retry', async () => {
    const failure = new Error('initialization failed');
    sdk.init.mockImplementationOnce(() => {
      throw failure;
    });
    const transport = createBrowserAnalyticsTransport(options);
    await expect(transport.setErrorReportingEnabled?.(true)).rejects.toBe(failure);
    await transport.setErrorReportingEnabled?.(true);
    expect(sdk.init).toHaveBeenCalledTimes(2);
  });

  it.each([undefined, '', '   '])('never initializes with absent DSN %j', async (sentryDsn) => {
    const transport = createBrowserAnalyticsTransport({ ...options, sentryDsn });
    await transport.setErrorReportingEnabled?.(true);
    await transport.setErrorReportingEnabled?.(false);
    expect(sdk.init).not.toHaveBeenCalled();
    expect(sdk.close).not.toHaveBeenCalled();
  });

  it('does not load browser reporting in server rendering', async () => {
    vi.stubGlobal('window', undefined);
    const transport = createBrowserAnalyticsTransport(options);
    await transport.setErrorReportingEnabled?.(true);
    await transport.setProductAnalyticsEnabled?.(true);
    expect(sdk.init).not.toHaveBeenCalled();
  });

  it.each(['0', '0.25', '', 'invalid', 'Infinity', true, undefined])(
    'parses environment sample rate %j',
    async (value) => {
      const transport = createBrowserAnalyticsTransportFromEnv({
        appName: 'v1-runtime',
        environment: 'development',
        env: { VITE_SENTRY_DSN: options.sentryDsn, VITE_SENTRY_TRACES_SAMPLE_RATE: value },
      });
      await transport.setErrorReportingEnabled?.(true);
      expect(sdk.init).toHaveBeenCalledWith(
        expect.objectContaining({
          tracesSampleRate: value === '0' ? 0 : value === '0.25' ? 0.25 : 1,
        }),
      );
    },
  );

  it.each(['development', 'production'])(
    'reports missing analytics config only in %s development policy',
    async (environment) => {
      const log = vi.spyOn(console, 'error').mockImplementation(() => undefined);
      const transport = createBrowserAnalyticsTransportFromEnv({
        appName: 'v1-runtime',
        environment,
        env: {},
      });
      await transport.setProductAnalyticsEnabled?.(true);
      expect(log).toHaveBeenCalledTimes(environment === 'development' ? 1 : 0);
      expect(sdk.init).not.toHaveBeenCalled();
    },
  );
});
