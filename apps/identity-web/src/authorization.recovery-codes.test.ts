import { afterEach, expect, it, vi } from 'vite-plus/test';
import { AuthorizationController } from './authorization.controller';
import { identityGateway } from './authorization.gateway';

afterEach(() => vi.useRealTimers());
async function setup(strong = true) {
  const gateway = identityGateway('https://identity.example');
  vi.spyOn(gateway, 'load').mockResolvedValue({
    interaction: 'id',
    csrfToken: 'csrf',
    sessionCsrfToken: 'session',
    needsLogin: false,
    clientId: 'account',
    scope: 'openid',
  });
  vi.spyOn(gateway, 'status').mockResolvedValue({
    minimumAuthentication: strong ? 'recent_mfa' : 'primary',
    needsLogin: false,
    needsStepUp: false,
    proofExpiresAt: strong ? new Date(Date.now() + 60_000).toISOString() : null,
  });
  vi.spyOn(gateway, 'hasFactors').mockResolvedValue(true);
  const generate = vi.spyOn(gateway, 'recoveryCodes').mockResolvedValue(['synthetic']);
  const controller = new AuthorizationController(gateway, vi.fn());
  await controller.start('authorization');
  return { controller, generate };
}

it('requires fresh strong proof and never generates from primary-only consent', async () => {
  const { controller, generate } = await setup(false);
  await controller.generateRecoveryCodes();
  expect(generate).not.toHaveBeenCalled();
});

it('generates once, blocks approval during display, and clears codes on acknowledgement', async () => {
  const { controller, generate } = await setup();
  await Promise.all([controller.generateRecoveryCodes(), controller.generateRecoveryCodes()]);
  expect(generate).toHaveBeenCalledExactlyOnceWith('session');
  expect(controller.snapshot().recoveryCodes).toEqual(['synthetic']);
  await controller.consent('approve');
  expect(controller.snapshot().stage).toBe('recovery-codes');
  await controller.dismissRecoveryCodes();
  expect(controller.snapshot().recoveryCodes).toBeNull();
  expect(controller.snapshot().stage).toBe('consent');
});

it('clears displayed codes when the proof expires', async () => {
  vi.useFakeTimers();
  const { controller } = await setup();
  await controller.generateRecoveryCodes();
  vi.advanceTimersByTime(60_000);
  controller.expireRecoveryCodes();
  expect(controller.snapshot().stage).toBe('closed');
  expect(controller.snapshot().recoveryCodes).toBeNull();
});

it('discards a late generation response after leaving the page', async () => {
  const { controller, generate } = await setup();
  let finish: (codes: string[]) => void = () => {};
  generate.mockReturnValue(
    new Promise((resolve) => {
      finish = resolve;
    }),
  );
  const pending = controller.generateRecoveryCodes();
  controller.dispose();
  finish(['synthetic']);
  await pending;
  expect(controller.snapshot().recoveryCodes).toBeNull();
  expect(controller.snapshot().stage).toBe('closed');
});

it('does not retry uncertain generation, which may have invalidated previous codes', async () => {
  const { controller, generate } = await setup();
  generate.mockRejectedValue(new TypeError('Network failure'));
  await controller.generateRecoveryCodes();
  await controller.generateRecoveryCodes();
  expect(generate).toHaveBeenCalledTimes(1);
  expect(controller.snapshot().stage).toBe('closed');
});
