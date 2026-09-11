import { afterEach, expect, it, vi } from 'vite-plus/test';
import {
  HostedIdentityError,
  type HostedAuthenticationStatus,
} from '@nvbes/identity-sdk-web/oauth';
import { AuthorizationController } from './authorization.controller';
import { identityGateway } from './authorization.gateway';

afterEach(() => vi.useRealTimers());
function setup(policy: HostedAuthenticationStatus['minimumAuthentication'] = 'recent_mfa') {
  const gateway = identityGateway('https://identity.example');
  const interaction = {
    interaction: 'id',
    csrfToken: 'csrf',
    sessionCsrfToken: 'session',
    needsLogin: false,
    clientId: 'account',
    scope: 'openid',
  };
  vi.spyOn(gateway, 'load').mockResolvedValue(interaction);
  const status = vi.spyOn(gateway, 'status').mockResolvedValue({
    minimumAuthentication: policy,
    needsLogin: false,
    needsStepUp: true,
    proofExpiresAt: null,
  });
  const hasFactors = vi.spyOn(gateway, 'hasFactors').mockResolvedValue(false);
  const hasTotp = vi.spyOn(gateway, 'hasTotp').mockResolvedValue(false);
  const register = vi.spyOn(gateway, 'registerPasskey').mockResolvedValue('credential');
  const stepUp = vi.spyOn(gateway, 'stepUpPasskey').mockResolvedValue('2030-01-01T00:00:00Z');
  const material = {
    factorId: 'factor',
    secretBase32: 'SYNTHETIC',
    provisioningUri: 'test-only',
    expiresAt: new Date(Date.now() + 60_000).toISOString(),
  };
  const startTotp = vi.spyOn(gateway, 'startTotp').mockResolvedValue(material);
  const confirmTotp = vi.spyOn(gateway, 'confirmTotp').mockResolvedValue('2030-01-01T00:00:00Z');
  const controller = new AuthorizationController(gateway, vi.fn());
  return {
    controller,
    status,
    hasFactors,
    hasTotp,
    register,
    stepUp,
    startTotp,
    confirmTotp,
    material,
  };
}

it('enrolls the first passkey and requires a separate assertion before consent', async () => {
  const test = setup('recent_webauthn');
  await test.controller.start('authorization');
  expect(test.controller.snapshot().stage).toBe('enrollment');
  test.hasFactors.mockResolvedValue(true);
  test.status.mockResolvedValue({
    minimumAuthentication: 'recent_webauthn',
    needsLogin: false,
    needsStepUp: false,
    proofExpiresAt: '2030-01-01T00:00:00Z',
  });
  await test.controller.registerPasskey('My key');
  expect(test.register).toHaveBeenCalledExactlyOnceWith('session', 'My key');
  expect(test.stepUp).toHaveBeenCalledExactlyOnceWith('session');
  expect(test.controller.snapshot().stage).toBe('consent');
  expect(test.controller.snapshot().firstEnrollmentAvailable).toBe(false);
});

it('never enrolls TOTP to satisfy a WebAuthn-only requirement', async () => {
  const test = setup('recent_webauthn');
  await test.controller.start('authorization');
  await test.controller.startTotp();
  expect(test.startTotp).not.toHaveBeenCalled();
});

it('retains TOTP material on a rejected code and erases it after server confirmation', async () => {
  const test = setup();
  await test.controller.start('authorization');
  await Promise.all([test.controller.startTotp(), test.controller.startTotp()]);
  expect(test.startTotp).toHaveBeenCalledTimes(1);
  test.confirmTotp.mockRejectedValueOnce(new HostedIdentityError(400));
  await test.controller.confirmTotp('000000');
  expect(test.controller.snapshot().stage).toBe('enrollment');
  expect(test.controller.snapshot().totpEnrollment).toEqual(test.material);
  test.hasFactors.mockResolvedValue(true);
  test.status.mockResolvedValue({
    minimumAuthentication: 'recent_mfa',
    needsLogin: false,
    needsStepUp: false,
    proofExpiresAt: '2030-01-01T00:00:00Z',
  });
  await test.controller.confirmTotp('123456');
  expect(test.confirmTotp).toHaveBeenLastCalledWith('session', 'factor', '123456');
  expect(test.controller.snapshot().totpEnrollment).toBeNull();
  expect(test.controller.snapshot().stage).toBe('consent');
});

it('erases expired material and does not submit its confirmation', async () => {
  vi.useFakeTimers();
  const test = setup();
  await test.controller.start('authorization');
  await test.controller.startTotp();
  vi.advanceTimersByTime(60_000);
  await test.controller.confirmTotp('123456');
  expect(test.confirmTotp).not.toHaveBeenCalled();
  expect(test.controller.snapshot().totpEnrollment).toBeNull();
});

it('does not restore a secret received after leaving the document', async () => {
  const test = setup();
  await test.controller.start('authorization');
  let resolve: (material: typeof test.material) => void = () => {};
  test.startTotp.mockReturnValue(
    new Promise((done) => {
      resolve = done;
    }),
  );
  const pending = test.controller.startTotp();
  test.controller.dispose();
  resolve(test.material);
  await pending;
  expect(test.controller.snapshot().totpEnrollment).toBeNull();
  expect(test.controller.snapshot().stage).toBe('closed');
});

it('does not offer first enrollment when factors already exist', async () => {
  const test = setup();
  test.hasFactors.mockResolvedValue(true);
  await test.controller.start('authorization');
  test.controller.beginEnrollment();
  await test.controller.registerPasskey('another');
  await test.controller.startTotp();
  expect(test.controller.snapshot().stage).toBe('step-up');
  expect(test.register).not.toHaveBeenCalled();
  expect(test.startTotp).not.toHaveBeenCalled();
});

it('does not spend factor-list quotas after the server confirms fresh strong proof', async () => {
  const test = setup();
  test.status.mockResolvedValue({
    minimumAuthentication: 'recent_mfa',
    needsLogin: false,
    needsStepUp: false,
    proofExpiresAt: '2030-01-01T00:00:00Z',
  });
  await test.controller.start('authorization');
  expect(test.hasFactors).not.toHaveBeenCalled();
  expect(test.controller.snapshot().stage).toBe('consent');
  expect(test.controller.snapshot().firstEnrollmentAvailable).toBe(false);
});

it('adds another passkey only after fresh strong proof', async () => {
  const test = setup();
  test.status.mockResolvedValue({
    minimumAuthentication: 'recent_mfa',
    needsLogin: false,
    needsStepUp: false,
    proofExpiresAt: new Date(Date.now() + 60_000).toISOString(),
  });
  await test.controller.start('authorization');
  await test.controller.beginEnrollment();
  expect(test.controller.snapshot().stage).toBe('enrollment');
  expect(test.controller.snapshot().firstEnrollmentAvailable).toBe(false);
  await test.controller.registerPasskey('Backup key');
  expect(test.register).toHaveBeenCalledExactlyOnceWith('session', 'Backup key');
  expect(test.controller.snapshot().stage).toBe('consent');
});

it('does not offer another TOTP configuration when one is already active', async () => {
  const test = setup();
  test.status.mockResolvedValue({
    minimumAuthentication: 'recent_mfa',
    needsLogin: false,
    needsStepUp: false,
    proofExpiresAt: new Date(Date.now() + 60_000).toISOString(),
  });
  test.hasTotp.mockResolvedValue(true);
  await test.controller.start('authorization');
  await test.controller.beginEnrollment();
  await test.controller.startTotp();
  expect(test.startTotp).not.toHaveBeenCalled();
});

it('retains a supplementary TOTP secret after rejected confirmation without treating it as WebAuthn', async () => {
  const test = setup('recent_webauthn');
  test.status.mockResolvedValue({
    minimumAuthentication: 'recent_webauthn',
    needsLogin: false,
    needsStepUp: false,
    proofExpiresAt: new Date(Date.now() + 60_000).toISOString(),
  });
  await test.controller.start('authorization');
  await test.controller.beginEnrollment();
  await test.controller.startTotp();
  test.confirmTotp.mockRejectedValueOnce(new HostedIdentityError(400));
  await test.controller.confirmTotp('000000');
  expect(test.controller.snapshot().stage).toBe('enrollment');
  expect(test.controller.snapshot().totpEnrollment).toEqual(test.material);
  test.status.mockResolvedValue({
    minimumAuthentication: 'recent_webauthn',
    needsLogin: false,
    needsStepUp: true,
    proofExpiresAt: null,
  });
  test.hasFactors.mockResolvedValue(true);
  await test.controller.confirmTotp('123456');
  expect(test.controller.snapshot().stage).toBe('step-up');
});

it('refuses a supplementary enrollment after its strong proof expires', async () => {
  vi.useFakeTimers();
  const test = setup();
  test.status.mockResolvedValue({
    minimumAuthentication: 'recent_mfa',
    needsLogin: false,
    needsStepUp: false,
    proofExpiresAt: new Date(Date.now() + 60_000).toISOString(),
  });
  await test.controller.start('authorization');
  await test.controller.beginEnrollment();
  vi.advanceTimersByTime(60_000);
  await test.controller.registerPasskey('key');
  await test.controller.startTotp();
  expect(test.register).not.toHaveBeenCalled();
  expect(test.startTotp).not.toHaveBeenCalled();
});
