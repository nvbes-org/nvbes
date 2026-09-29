import { afterEach, beforeEach, expect, it, vi } from 'vite-plus/test';
import { getHostedAuthenticationStatus } from '../hosted.authentication';
import { parseHostedInteraction } from '../hosted.client';

const baseUrl = 'https://identity.example';
const interaction = parseHostedInteraction({
  interaction: 'bound-interaction',
  csrf_token: 'interaction-proof',
  session_csrf_token: 'session-proof',
  needs_login: false,
  client_id: 'account-web',
  scope: 'openid account:read',
});
const required = {
  minimum_authentication: 'recent_webauthn',
  needs_login: false,
  needs_step_up: true,
  proof_expires_at: null,
};
const json = (value: unknown, status = 200) =>
  new Response(JSON.stringify(value), { status, headers: { 'content-type': 'application/json' } });
beforeEach(() => vi.stubGlobal('location', { origin: baseUrl }));
afterEach(() => vi.unstubAllGlobals());

it('reads fresh server policy with interaction CSRF without mutating local authentication state', async () => {
  const fetchImpl = vi
    .fn<typeof fetch>()
    .mockResolvedValueOnce(json(required))
    .mockResolvedValueOnce(
      json({ ...required, needs_step_up: false, proof_expires_at: '2030-01-01T00:00:00Z' }),
    );
  expect(await getHostedAuthenticationStatus({ baseUrl, fetchImpl }, interaction)).toEqual({
    minimumAuthentication: 'recent_webauthn',
    needsLogin: false,
    needsStepUp: true,
    proofExpiresAt: null,
  });
  expect(
    (await getHostedAuthenticationStatus({ baseUrl, fetchImpl }, interaction)).needsStepUp,
  ).toBe(false);
  expect(interaction.sessionCsrfToken).toBe('session-proof');
  for (const [url, init] of fetchImpl.mock.calls) {
    expect(url).toBe(`${baseUrl}/oauth/authorize/authentication`);
    expect(init).toMatchObject({
      method: 'POST',
      credentials: 'same-origin',
      cache: 'no-store',
      redirect: 'error',
    });
    expect(new Headers(init?.headers).get('x-csrf-token')).toBe('interaction-proof');
    expect(new Headers(init?.headers).has('authorization')).toBe(false);
    if (typeof init?.body !== 'string') throw new Error('Expected a JSON request body.');
    expect(JSON.parse(init.body)).toEqual({ interaction: 'bound-interaction' });
  }
});

it('rejects unknown policy, inconsistent states and absent or malformed proof expiry', async () => {
  for (const value of [
    { ...required, minimum_authentication: 'anything' },
    { ...required, needs_login: true },
    { ...required, minimum_authentication: 'primary' },
    { ...required, needs_step_up: 'false' },
    { ...required, needs_step_up: false },
    { ...required, proof_expires_at: 'secret-invalid-date' },
    { ...required, needs_step_up: false, proof_expires_at: undefined },
  ]) {
    const fetchImpl = vi.fn<typeof fetch>().mockResolvedValueOnce(json(value));
    await expect(
      getHostedAuthenticationStatus({ baseUrl, fetchImpl }, interaction),
    ).rejects.toThrow();
    expect(fetchImpl).toHaveBeenCalledTimes(1);
  }
});

it('propagates refusal and outage without retry or a fabricated satisfied state', async () => {
  for (const status of [400, 403, 429, 503]) {
    const fetchImpl = vi
      .fn<typeof fetch>()
      .mockResolvedValueOnce(json({ error: 'temporarily_unavailable' }, status));
    await expect(
      getHostedAuthenticationStatus({ baseUrl, fetchImpl }, interaction),
    ).rejects.toThrow();
    expect(fetchImpl).toHaveBeenCalledTimes(1);
  }
});
