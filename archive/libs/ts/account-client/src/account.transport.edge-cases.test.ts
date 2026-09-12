import { z } from 'zod';
import { describe, expect, it, vi } from 'vite-plus/test';
import { AccountAuthenticationError, AccountHttpError } from './account.errors';
import { AccountTransport } from './account.transport';

describe('AccountTransport edge cases', () => {
  it('does not consume a 204 response body', async () => {
    const response = new Response(null, { status: 204 });
    const read = vi.spyOn(response, 'text');
    await expect(
      transportReturning(response).request('/empty', z.undefined(), { method: 'DELETE' }),
    ).resolves.toBeUndefined();
    expect(read).not.toHaveBeenCalled();
  });

  it('preserves internal separators in a relative route without a leading slash', async () => {
    const fetchImpl = vi.fn<typeof fetch>().mockResolvedValue(Response.json({ ok: true }));
    const transport = new AccountTransport({
      baseUrl: 'https://account.test/api/',
      fetchImpl,
      getAccessToken: () => 'synthetic',
    });
    await transport.request('nested//resource', z.object({ ok: z.boolean() }), { method: 'GET' });
    expect(fetchImpl.mock.calls[0]?.[0]).toBe('https://account.test/api/nested//resource');
  });

  it('rejects a whitespace-only token before invoking fetch', async () => {
    let called = false;
    const transport = new AccountTransport({
      fetchImpl: async () => {
        called = true;
        return new Response();
      },
      getAccessToken: () => '   ',
    });

    const error = await transport
      .request('/profile', z.unknown(), { method: 'GET' })
      .catch((cause: unknown) => cause);
    expect(error).toBeInstanceOf(AccountAuthenticationError);
    expect(error).toMatchObject({
      code: 'missing_access_token',
      message: 'An Account OAuth access token is required',
      name: 'AccountAuthenticationError',
    });
    expect(called).toBe(false);
  });

  it('uses the status fallback and response request id for a non-JSON failure', async () => {
    const transport = transportReturning(
      new Response('temporarily unavailable', {
        headers: { 'x-request-id': 'request-from-header' },
        status: 503,
        statusText: 'Service Unavailable',
      }),
    );

    const error = await transport
      .request('/profile', z.unknown(), { method: 'GET' })
      .catch((cause: unknown) => cause);

    expect(error).toBeInstanceOf(AccountHttpError);
    expect(error).toMatchObject({
      body: 'temporarily unavailable',
      message: '503 Service Unavailable',
      name: 'AccountHttpError',
      requestId: 'request-from-header',
      status: 503,
      statusText: 'Service Unavailable',
    });
  });

  it('accepts an empty successful response when the DTO allows undefined', async () => {
    const transport = transportReturning(new Response(null, { status: 200 }));
    await expect(transport.request('/empty', z.undefined(), { method: 'DELETE' })).resolves.toBe(
      undefined,
    );
  });

  it('falls back safely when a JSON error envelope has no error member', async () => {
    const transport = transportReturning(
      Response.json(
        {},
        {
          headers: { 'x-request-id': 'fallback-request' },
          status: 400,
          statusText: 'Bad Request',
        },
      ),
    );
    await expect(
      transport.request('/profile', z.unknown(), { method: 'GET' }),
    ).rejects.toMatchObject({
      message: '400 Bad Request',
      requestId: 'fallback-request',
    });
  });

  it('validates blob failures before returning their response body', async () => {
    const failure = transportReturning(
      Response.json(
        { error: { message: 'Archive expired', request_id: 'export-request' } },
        { status: 410 },
      ),
    );
    await expect(failure.requestBlob('/export')).rejects.toMatchObject({
      message: 'Archive expired',
      requestId: 'export-request',
      status: 410,
    });
  });

  it('normalizes boundary slashes without altering inner path slashes', async () => {
    let requestedUrl = '';
    const transport = new AccountTransport({
      baseUrl: 'https://account.example.test/root///',
      fetchImpl: async (input) => {
        requestedUrl =
          typeof input === 'string' ? input : input instanceof URL ? input.toString() : input.url;
        return Response.json({ ok: true });
      },
      getAccessToken: () => 'token',
    });

    await transport.request('///nested//resource', z.object({ ok: z.boolean() }), {
      method: 'GET',
    });
    expect(requestedUrl).toBe('https://account.example.test/root/nested//resource');
  });
});

function transportReturning(response: Response): AccountTransport {
  return new AccountTransport({
    baseUrl: 'https://account.example.test',
    fetchImpl: async () => response,
    getAccessToken: () => 'token',
  });
}
