import { afterEach, expect, it, vi } from 'vite-plus/test';
import { AuthorizationController } from './authorization.controller';
import { identityGateway } from './authorization.gateway';
import { managementExpiry } from './authorization.state';

afterEach(() => vi.useRealTimers());
async function setup() {
  const gateway = identityGateway('https://identity.example');
  vi.spyOn(gateway, 'load').mockResolvedValue({
    interaction: 'id',
    csrfToken: 'csrf',
    sessionCsrfToken: 'session',
    needsLogin: false,
    clientId: 'client',
    scope: 'openid',
  });
  const status = {
    minimumAuthentication: 'primary' as const,
    needsLogin: false,
    needsStepUp: false,
    proofExpiresAt: null,
  };
  vi.spyOn(gateway, 'status').mockResolvedValue(status);
  vi.spyOn(gateway, 'hasFactors').mockResolvedValue(true);
  const passkey = vi
    .spyOn(gateway, 'stepUpPasskey')
    .mockResolvedValue(new Date(Date.now() + 600_000).toISOString());
  const totp = vi
    .spyOn(gateway, 'stepUpTotp')
    .mockResolvedValue(new Date(Date.now() + 600_000).toISOString());
  const controller = new AuthorizationController(gateway, vi.fn());
  await controller.start('authorization');
  return { controller, passkey, totp, status };
}

it.each(['passkey', 'totp'] as const)(
  'opens security with explicit %s without changing OAuth policy',
  async (method) => {
    const test = await setup();
    test.controller.beginFactors();
    expect(test.controller.snapshot().stage).toBe('consent');
    test.controller.beginSecurityStepUp();
    expect(test.controller.snapshot().stage).toBe('security-step-up');
    if (method === 'passkey') await test.controller.passkey();
    else await test.controller.totp('123456');
    expect(test.controller.snapshot().authentication).toEqual(test.status);
    expect(managementExpiry(test.controller.snapshot())).not.toBeNull();
    test.controller.beginFactors();
    expect(test.controller.snapshot().stage).toBe('factors');
  },
);

it('bounds management freshness to five minutes and permits a new explicit challenge', async () => {
  vi.useFakeTimers();
  const test = await setup();
  test.controller.beginSecurityStepUp();
  await test.controller.passkey();
  vi.advanceTimersByTime(300_000);
  expect(managementExpiry(test.controller.snapshot())).toBeNull();
  test.controller.beginFactors();
  expect(test.controller.snapshot().stage).toBe('consent');
  test.controller.beginSecurityStepUp();
  expect(test.controller.snapshot().stage).toBe('security-step-up');
});

it('cancels optional step-up without issuing a security proof', async () => {
  const test = await setup();
  test.controller.beginSecurityStepUp();
  await test.controller.cancelSecurityStepUp();
  expect(test.controller.snapshot().stage).toBe('consent');
  expect(managementExpiry(test.controller.snapshot())).toBeNull();
  expect(test.passkey).not.toHaveBeenCalled();
  expect(test.totp).not.toHaveBeenCalled();
});

it('does not restore management proof after a late successful assertion on page exit', async () => {
  const test = await setup();
  let finish: (expiry: string) => void = () => {};
  test.passkey.mockReturnValue(
    new Promise((resolve) => {
      finish = resolve;
    }),
  );
  test.controller.beginSecurityStepUp();
  const pending = test.controller.passkey();
  test.controller.dispose();
  finish(new Date(Date.now() + 600_000).toISOString());
  await pending;
  expect(managementExpiry(test.controller.snapshot())).toBeNull();
  expect(test.controller.snapshot().stage).toBe('closed');
});
