import { afterEach, expect, it, vi } from 'vitest';
import {
  loadPasswordRecovery,
  requestPasswordRecovery,
  resetRecoveredPassword,
} from '../hosted.password-recovery';

afterEach(() => vi.unstubAllGlobals());
const baseUrl = 'https://identity.example';
it('validates acknowledgements and sends reset secrets only in the POST body', async () => {
  vi.stubGlobal('location', { origin: baseUrl });
  const fetchImpl = vi
    .fn<typeof fetch>()
    .mockResolvedValueOnce(Response.json({ csrf_token: 'c'.repeat(43) }))
    .mockResolvedValueOnce(Response.json({ accepted: true }, { status: 202 }))
    .mockResolvedValueOnce(Response.json({ reset: true, must_reauthenticate: true }));
  const config = { baseUrl, fetchImpl };
  const csrf = await loadPasswordRecovery(config);
  await requestPasswordRecovery(config, csrf, 'person@example.invalid');
  await resetRecoveredPassword(config, csrf, 't'.repeat(43), 'new-password!');
  expect(fetchImpl).toHaveBeenCalledTimes(3);
  const [url, request] = fetchImpl.mock.calls[2];
  expect(url).toBe(baseUrl + '/oauth/password-recovery/reset');
  expect(request?.method).toBe('POST');
  expect(request?.redirect).toBe('error');
  expect(new Headers(request?.headers).get('x-csrf-token')).toBe(csrf);
  expect(JSON.parse(String(request?.body))).toEqual({
    token: 't'.repeat(43),
    password: 'new-password!',
  });
});
it('rejects absent or false server acknowledgements without retrying', async () => {
  vi.stubGlobal('location', { origin: baseUrl });
  for (const result of [{ reset: false, must_reauthenticate: true }, { reset: true }, {}]) {
    const fetchImpl = vi.fn<typeof fetch>().mockResolvedValue(Response.json(result));
    await expect(
      resetRecoveredPassword(
        { baseUrl, fetchImpl },
        'c'.repeat(43),
        't'.repeat(43),
        'new-password!',
      ),
    ).rejects.toThrow();
    expect(fetchImpl).toHaveBeenCalledTimes(1);
  }
});
it('refuses another browser origin before transmitting a secret', async () => {
  vi.stubGlobal('location', { origin: 'https://attacker.example' });
  const fetchImpl = vi.fn<typeof fetch>();
  await expect(
    resetRecoveredPassword({ baseUrl, fetchImpl }, 'c'.repeat(43), 't'.repeat(43), 'new-password!'),
  ).rejects.toThrow();
  expect(fetchImpl).not.toHaveBeenCalled();
});
