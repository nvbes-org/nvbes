import { afterEach, describe, expect, it, vi } from 'vite-plus/test';
import { z } from 'zod';
import { createHttpClient, DtoValidationError, HttpError, resolveRequestUrl } from './index';

afterEach(() => {
  vi.restoreAllMocks();
});

describe('HttpClient response contract', () => {
  it('parses JSON, text, empty and no-content responses through the supplied DTO', async () => {
    const responses = [
      response(JSON.stringify({ value: 42 }), 200, 'application/json'),
      response('plain text', 200, 'text/plain'),
      response('', 200, 'text/plain'),
      new Response(null, { status: 204 }),
    ];
    const client = createHttpClient({
      baseUrl: 'https://account.example.test',
      fetchImpl: queuedFetch(responses),
    });

    await expect(client.get('/json', z.object({ value: z.literal(42) }))).resolves.toEqual({
      value: 42,
    });
    await expect(client.get('/text', z.literal('plain text'))).resolves.toBe('plain text');
    await expect(client.get('/empty', z.undefined())).resolves.toBeUndefined();
    await expect(client.delete('/no-content', z.undefined())).resolves.toBeUndefined();
  });

  it('surfaces structured API errors including recovery and envelope request id', async () => {
    const client = createHttpClient({
      fetchImpl: queuedFetch([
        new Response(
          JSON.stringify({
            error: {
              message: 'Session expired',
              recovery: 'reauthenticate',
              request_id: 'request-from-envelope',
            },
          }),
          {
            headers: { 'Content-Type': 'application/json', 'X-Request-Id': 'header-request' },
            status: 401,
            statusText: 'Unauthorized',
          },
        ),
      ]),
    });

    const error = await client.get('/auth/me', z.unknown()).catch((cause: unknown) => cause);

    expect(error).toBeInstanceOf(HttpError);
    expect(error).toMatchObject({
      body: {
        error: {
          message: 'Session expired',
          recovery: 'reauthenticate',
          request_id: 'request-from-envelope',
        },
      },
      message: 'Session expired',
      recovery: 'reauthenticate',
      requestId: 'request-from-envelope',
      status: 401,
      statusText: 'Unauthorized',
    });
  });

  it('falls back to status text and the response request id for non-envelope failures', async () => {
    const client = createHttpClient({
      fetchImpl: queuedFetch([
        new Response('upstream unavailable', {
          headers: { 'X-Request-Id': 'request-from-header' },
          status: 502,
          statusText: 'Bad Gateway',
        }),
      ]),
    });

    const error = await client.get('/health', z.unknown()).catch((cause: unknown) => cause);

    expect(error).toMatchObject({
      body: 'upstream unavailable',
      message: '502 Bad Gateway',
      recovery: undefined,
      requestId: 'request-from-header',
      status: 502,
    });
  });

  it('wraps DTO mismatches and emits a diagnostic without accepting malformed data', async () => {
    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => undefined);
    const client = createHttpClient({
      fetchImpl: queuedFetch([response(JSON.stringify({ value: 'super-secret-response-value' }))]),
    });

    const error = await client
      .get('/typed', z.object({ value: z.number() }))
      .catch((cause: unknown) => cause);

    expect(error).toBeInstanceOf(DtoValidationError);
    expect((error as DtoValidationError).cause.issues[0]?.path).toEqual(['value']);
    expect(consoleError).toHaveBeenCalledWith(
      'DTO validation failed',
      (error as DtoValidationError).cause.issues,
    );
    expect(JSON.stringify(consoleError.mock.calls)).not.toContain('super-secret-response-value');
  });
});

describe('resolveRequestUrl', () => {
  it.each([
    ['jobs', 'https://api.example.test/v1/', 'https://api.example.test/v1/jobs'],
    ['/jobs', 'https://api.example.test/v1', 'https://api.example.test/v1/jobs'],
    ['/jobs', 'https://api.example.test', 'https://api.example.test/jobs'],
    [
      'https://uploads.example.test/object',
      'https://api.example.test/v1',
      'https://uploads.example.test/object',
    ],
  ])('resolves %s against %s', (path, baseUrl, expected) => {
    expect(resolveRequestUrl(path, baseUrl).toString()).toBe(expected);
  });
});

function queuedFetch(responses: Response[]): typeof fetch {
  return async () => {
    const next = responses.shift();
    if (!next) {
      throw new Error('Unexpected fetch call');
    }
    return next;
  };
}

function response(body: string, status = 200, contentType = 'application/json'): Response {
  return new Response(body, {
    headers: { 'Content-Type': contentType },
    status,
  });
}
