import { DtoValidationError, HttpError } from '@nvbes/http-client';
import { Effect } from 'effect';
import { describe, expect, it } from 'vite-plus/test';
import {
  ClientRuntimeError,
  effectMutationFn,
  effectQueryFn,
  normalizeClientError,
  sanitizeUrlString,
  scrubReplayRecordingEvent,
  scrubSentryBreadcrumb,
  scrubSentryEvent,
} from './index';
import { z } from 'zod';

describe('web-runtime Effect adapters', () => {
  it('runs a successful Effect as a query function', async () => {
    const queryFn = effectQueryFn(() => Effect.succeed('ok'));

    await expect(
      queryFn({
        queryKey: ['test'],
        client: {} as never,
        signal: new AbortController().signal,
        meta: undefined,
      }),
    ).resolves.toBe('ok');
  });

  it('runs a successful Effect as a mutation function', async () => {
    const mutationFn = effectMutationFn((value: string) => Effect.succeed(value.toUpperCase()));

    await expect(mutationFn('nvbes', {} as never)).resolves.toBe('NVBES');
  });

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

describe('Sentry privacy scrubbing', () => {
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
