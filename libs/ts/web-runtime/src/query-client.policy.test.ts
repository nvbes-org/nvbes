import { HttpError } from '@nvbes/http-client';
import { keepPreviousData } from '@tanstack/react-query';
import { describe, expect, it, vi } from 'vite-plus/test';
import { ClientRuntimeError, createQueryClient, normalizeClientError } from './index';

describe('query client recovery policy', () => {
  it('sets bounded cache lifetimes and never automatically retries mutations', () => {
    const client = createQueryClient();
    expect(client.getDefaultOptions()).toEqual({
      queries: {
        gcTime: 1_800_000,
        staleTime: 30_000,
        placeholderData: keepPreviousData,
        retry: expect.any(Function),
        refetchOnWindowFocus: false,
        throwOnError: false,
      },
      mutations: { retry: false },
    });
    client.clear();
  });

  it.each([400, 401, 403, 404, 409, 429, 499])('does not retry HTTP %i', async (status) => {
    const error = new HttpError('rejected', new Response(null, { status }), {});
    const queryFn = vi.fn().mockRejectedValue(error);
    const client = createQueryClient();
    try {
      await expect(
        client.fetchQuery({ queryKey: ['profile'], queryFn, retryDelay: 0 }),
      ).rejects.toBe(error);
      expect(queryFn).toHaveBeenCalledTimes(1);
    } finally {
      client.clear();
    }
  });

  it.each([
    new Error('offline'),
    new HttpError('unavailable', new Response(null, { status: 500 }), {}),
  ])('bounds transient failure retries at three attempts', async (error) => {
    const queryFn = vi.fn().mockRejectedValue(error);
    const client = createQueryClient();
    try {
      await expect(
        client.fetchQuery({ queryKey: ['profile'], queryFn, retryDelay: 0 }),
      ).rejects.toBe(error);
      expect(queryFn).toHaveBeenCalledTimes(3);
    } finally {
      client.clear();
    }
  });

  it('does not retry an explicit reauthentication instruction even on a server error', async () => {
    const error = new HttpError('expired', new Response(null, { status: 503 }), {
      error: { recovery: 'reauthenticate' },
    });
    const client = createQueryClient();
    const queryFn = vi.fn().mockRejectedValue(error);
    try {
      await expect(
        client.fetchQuery({ queryKey: ['session'], queryFn, retryDelay: 0 }),
      ).rejects.toBe(error);
      expect(queryFn).toHaveBeenCalledTimes(1);
    } finally {
      client.clear();
    }
  });

  it('preserves normalized errors and their correlation data', () => {
    const cause = new Error('source');
    const error = new ClientRuntimeError(
      'api',
      'denied',
      cause,
      403,
      { code: 'denied' },
      'req-test',
    );
    expect(normalizeClientError(error)).toBe(error);
    expect(error).toMatchObject({
      name: 'ClientRuntimeError',
      kind: 'api',
      message: 'denied',
      cause,
      status: 403,
      body: { code: 'denied' },
      requestId: 'req-test',
    });
    expect(normalizeClientError(cause)).toMatchObject({
      kind: 'unexpected',
      cause,
      message: 'source',
    });
  });
});
