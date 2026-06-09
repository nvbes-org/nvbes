import { DtoValidationError, HttpError } from '@nvbes/http-client';
import { afterEach, describe, expect, it } from 'vite-plus/test';
import {
  ClientRuntimeError,
  createSentryFeedbackOptions,
  installBrowserSentrySmoke,
  isBrowserSentrySmokeEnabled,
  normalizeClientError,
  SENTRY_SMOKE_GLOBAL,
  sanitizeUrlString,
  scrubReplayRecordingEvent,
  scrubSentryBreadcrumb,
  scrubSentryEvent,
} from './index';
import { z } from 'zod';

describe('web-runtime client errors', () => {
  it('normalizes HttpError', () => {
    const response = new Response(JSON.stringify({ error: { message: 'Nope' } }), {
      status: 401,
      statusText: 'Unauthorized',
    });
    const error = normalizeClientError(
      new HttpError('Nope', response, { error: { message: 'Nope' } }),
    );

    expect(error).toBeInstanceOf(ClientRuntimeError);
    expect(error.kind).toBe('api');
    expect(error.status).toBe(401);
  });

  it('normalizes DTO validation errors', () => {
    const parsed = z.object({ id: z.string() }).safeParse({ id: 1 });
    if (parsed.success) {
      throw new Error('Expected validation failure');
    }

    const error = normalizeClientError(new DtoValidationError(parsed.error));

    expect(error.kind).toBe('dto');
  });

  it('normalizes unknown failures', () => {
    const error = normalizeClientError('boom');

    expect(error.kind).toBe('unexpected');
  });
});

describe('Sentry browser smoke test', () => {
  afterEach(() => {
    Reflect.deleteProperty(globalThis, 'window');
  });

  it('is enabled in development or by explicit env flag', () => {
    expect(isBrowserSentrySmokeEnabled(undefined, true)).toBe(true);
    expect(isBrowserSentrySmokeEnabled('true', false)).toBe(true);
    expect(isBrowserSentrySmokeEnabled('1', false)).toBe(true);
    expect(isBrowserSentrySmokeEnabled('false', false)).toBe(false);
  });

  it('does not install the global smoke function when disabled', () => {
    installTestWindow();
    delete window[SENTRY_SMOKE_GLOBAL];

    const installed = installBrowserSentrySmoke({
      appName: 'drive-web',
      dsnConfigured: true,
      enabled: false,
      environment: 'test',
      initialized: true,
      reporter: {
        captureMessage: () => 'event-id',
        flush: async () => true,
        withScope: (callback) =>
          callback({
            setTag: () => undefined,
          }),
      },
      runtime: 'browser',
    });

    expect(installed).toBe(false);
    expect(window[SENTRY_SMOKE_GLOBAL]).toBeUndefined();
  });

  it('captures a low-PII browser smoke message when installed', async () => {
    installTestWindow();
    delete window[SENTRY_SMOKE_GLOBAL];

    const tags = new Map<string, string | boolean>();
    installBrowserSentrySmoke({
      appName: 'identity-web',
      dsnConfigured: true,
      enabled: true,
      environment: 'test',
      initialized: true,
      reporter: {
        captureMessage: (message, level) => `${level}:${message}`,
        flush: async () => true,
        withScope: (callback) =>
          callback({
            setTag: (key, value) => tags.set(key, value),
            setTransactionName: (name) => tags.set('transaction', name ?? ''),
          }),
      },
      runtime: 'browser',
    });

    const smoke = window[SENTRY_SMOKE_GLOBAL];
    expect(smoke).toBeDefined();
    const result = await smoke?.();

    expect(result).toMatchObject({
      appName: 'identity-web',
      eventId: 'info:nvbes browser sentry smoke test',
      flushed: true,
      status: 'accepted',
    });
    expect(tags.get('smoke_test')).toBe('sentry');
  });

  it('reports skipped when Sentry is not initialized', async () => {
    installTestWindow();
    delete window[SENTRY_SMOKE_GLOBAL];

    installBrowserSentrySmoke({
      appName: 'drive-web',
      dsnConfigured: true,
      enabled: true,
      environment: 'test',
      initialized: false,
      reporter: {
        captureMessage: () => {
          throw new Error('captureMessage should not be called');
        },
        flush: async () => true,
        withScope: (callback) =>
          callback({
            setTag: () => undefined,
          }),
      },
      runtime: 'browser',
    });

    await expect(window[SENTRY_SMOKE_GLOBAL]?.()).resolves.toMatchObject({
      eventId: null,
      reason: 'sentry_not_initialized',
      status: 'skipped',
    });
  });
});

function installTestWindow() {
  Object.defineProperty(globalThis, 'window', {
    configurable: true,
    value: {},
    writable: true,
  });
}

describe('Sentry privacy scrubbing', () => {
  it('keeps Sentry feedback explicit and low-PII', () => {
    expect(createSentryFeedbackOptions('drive-web')).toMatchObject({
      autoInject: true,
      showEmail: false,
      showName: false,
      enableScreenshot: false,
      tags: {
        app: 'drive-web',
        feature: 'user-feedback',
      },
    });
  });

  it('removes user data and request payloads from Sentry events', () => {
    const event = scrubSentryEvent({
      user: { email: 'ada@example.com' },
      message: 'Failed Bearer abcdefghijklmnopqrstuvwxyz',
      request: {
        url: 'https://api.nvbes.fr/users/ada@example.com?token=secret&cursor=abc',
        query_string: 'token=secret',
        cookies: 'sid=secret',
        data: '{"email":"ada@example.com"}',
        headers: {
          authorization: 'Bearer secret',
          'content-type': 'application/json',
        },
      },
    });

    expect(event).not.toBeNull();
    expect(event?.user).toBeUndefined();
    expect(event?.message).toBe(`Failed Bearer [Filtered]`);
    expect(event?.request.url).toBe(
      'https://api.nvbes.fr/users/[id]?token=%5BFiltered%5D&cursor=%5BFiltered%5D',
    );
    expect(event?.request.query_string).toBeUndefined();
    expect(event?.request.cookies).toBeUndefined();
    expect(event?.request.data).toBeUndefined();
    expect(event?.request.headers).toEqual({ 'content-type': 'application/json' });
  });

  it('drops console breadcrumbs and scrubs breadcrumb URLs', () => {
    expect(scrubSentryBreadcrumb({ category: 'console', message: 'ada@example.com' })).toBeNull();

    const breadcrumb = scrubSentryBreadcrumb({
      category: 'fetch',
      data: {
        url: '/files/report.pdf?token=secret',
        authorization: 'Bearer secret',
      },
    });

    expect(breadcrumb?.data).toEqual({
      url: '/files/[id]?token=%5BFiltered%5D',
      authorization: '[Filtered]',
    });
  });

  it('scrubs replay recording frames before they are buffered', () => {
    const frame = scrubReplayRecordingEvent({
      data: {
        payload: {
          url: '/account?email=ada@example.com',
          text: 'ada@example.com',
        },
      },
    });

    expect(frame?.data.payload).toEqual({
      url: '/account?email=%5BFiltered%5D',
      text: '[Filtered]',
    });
  });

  it('sanitizes URL path identifiers and query values', () => {
    expect(
      sanitizeUrlString(
        'https://api.nvbes.fr/workspaces/018f3c8a-7a37-7b1a-bc15-e8d2715c6c54/files/customer.xlsx?download_token=secret',
      ),
    ).toBe('https://api.nvbes.fr/workspaces/[id]/files/[id]?download_token=%5BFiltered%5D');
  });
});
