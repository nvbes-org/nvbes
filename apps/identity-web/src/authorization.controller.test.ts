import { describe, expect, it, vi } from 'vite-plus/test';
import {
  HostedIdentityError,
  WebauthnBrowserError,
  type HostedInteraction,
  type HostedAuthenticationStatus,
} from '@nvbes/identity-sdk-web/oauth';
import { AuthorizationController } from './authorization.controller';
import type { IdentityGateway } from './authorization.gateway';

const interaction: HostedInteraction = {
  interaction: 'interaction',
  csrfToken: 'first-csrf',
  sessionCsrfToken: null,
  needsLogin: true,
  clientId: 'account-web',
  scope: 'openid account:read',
};
const anonymous: HostedAuthenticationStatus = {
  minimumAuthentication: 'recent_mfa',
  needsLogin: true,
  needsStepUp: false,
  proofExpiresAt: null,
};
const loggedIn = {
  ...interaction,
  csrfToken: 'rotated-csrf',
  sessionCsrfToken: 'session-csrf',
  needsLogin: false,
};
const stepUp = { ...anonymous, needsLogin: false, needsStepUp: true };
const satisfied = {
  ...stepUp,
  needsStepUp: false,
  proofExpiresAt: '2030-01-01T00:00:00Z',
};

function setup() {
  const gateway = {
    load: vi.fn<IdentityGateway['load']>().mockResolvedValue(interaction),
    status: vi.fn<IdentityGateway['status']>().mockResolvedValue(anonymous),
    password: vi.fn<IdentityGateway['password']>().mockResolvedValue(loggedIn),
    passkey: vi.fn<IdentityGateway['passkey']>().mockResolvedValue(loggedIn),
    hasFactors: vi.fn<IdentityGateway['hasFactors']>().mockResolvedValue(true),
    recoveryCodes: vi.fn<IdentityGateway['recoveryCodes']>(),
    hasTotp: vi.fn<IdentityGateway['hasTotp']>().mockResolvedValue(false),
    redeemRecovery: vi.fn<IdentityGateway['redeemRecovery']>(),
    registerPasskey: vi.fn<IdentityGateway['registerPasskey']>(),
    startTotp: vi.fn<IdentityGateway['startTotp']>(),
    confirmTotp: vi.fn<IdentityGateway['confirmTotp']>(),
    stepUpPasskey: vi
      .fn<IdentityGateway['stepUpPasskey']>()
      .mockResolvedValue('2030-01-01T00:00:00Z'),
    stepUpTotp: vi.fn<IdentityGateway['stepUpTotp']>().mockResolvedValue('2030-01-01T00:00:00Z'),
    consent: vi
      .fn<IdentityGateway['consent']>()
      .mockResolvedValue('https://account.example/callback?code=opaque'),
  };
  const navigate = vi.fn();
  return {
    gateway,
    navigate,
    controller: new AuthorizationController(gateway, navigate),
  };
}

describe('hosted authorization orchestration', () => {
  it('reauthenticates a bound session and retains the rotated interaction proof', async () => {
    const { gateway, controller } = setup();
    gateway.load.mockResolvedValue(loggedIn);
    gateway.status.mockResolvedValue({
      ...anonymous,
      minimumAuthentication: 'primary',
      needsLogin: false,
    });
    gateway.hasFactors.mockResolvedValue(false);
    await controller.start('authorization');
    await controller.beginEnrollment();
    controller.beginReauthentication();
    expect(controller.snapshot().stage).toBe('reauthenticate');
    gateway.password.mockRejectedValueOnce(new HostedIdentityError(400));
    await controller.password('email', 'wrong');
    expect(controller.snapshot().stage).toBe('reauthenticate');
    gateway.password.mockResolvedValue({
      ...loggedIn,
      csrfToken: 'new-csrf',
      sessionCsrfToken: 'new-session',
    });
    await controller.password('email', 'correct');
    expect(controller.snapshot().stage).toBe('consent');
    expect(controller.snapshot().interaction?.csrfToken).toBe('new-csrf');
    expect(controller.snapshot().interaction?.sessionCsrfToken).toBe('new-session');
    await controller.beginEnrollment();
    await controller.startTotp();
    expect(gateway.startTotp).toHaveBeenCalledExactlyOnceWith('new-session');
  });

  it('abandons OAuth after recovery redemption and never approves the old interaction', async () => {
    const { gateway, controller, navigate } = setup();
    await controller.start('authorization');
    await controller.recover('code');
    expect(gateway.redeemRecovery).not.toHaveBeenCalled();
    gateway.status.mockResolvedValue(stepUp);
    await controller.password('email', 'password');
    gateway.redeemRecovery.mockResolvedValue({
      csrfToken: 'recovery-proof',
      expiresAt: '2030-01-01T00:00:00Z',
    });
    await controller.recover('code');
    expect(gateway.redeemRecovery).toHaveBeenCalledExactlyOnceWith('session-csrf', 'code');
    expect(controller.snapshot().interaction).toBeNull();
    expect(navigate).toHaveBeenCalledExactlyOnceWith('/recovery');
    await controller.consent('approve');
    expect(gateway.consent).not.toHaveBeenCalled();
  });

  it.each([
    'webauthn_not_allowed',
    'webauthn_timeout',
    'webauthn_not_supported',
    'webauthn_unsupported',
  ])('keeps the server-validated login usable after %s', async (code) => {
    const { gateway, controller } = setup();
    await controller.start('authorization');
    gateway.passkey.mockRejectedValue(new WebauthnBrowserError(code, 'private browser detail'));
    await controller.passkey();
    expect(controller.snapshot().stage).toBe('login');
    expect(controller.snapshot().error).not.toContain('private browser detail');
    expect(gateway.passkey).toHaveBeenCalledTimes(1);
    gateway.status.mockResolvedValue(stepUp);
    await controller.password('email', 'password');
    expect(controller.snapshot().stage).toBe('step-up');
    expect(gateway.password).toHaveBeenCalledTimes(1);
  });

  it('keeps unknown WebAuthn failures closed', async () => {
    const { gateway, controller } = setup();
    await controller.start('authorization');
    gateway.passkey.mockRejectedValue(
      new WebauthnBrowserError('webauthn_security_error', 'invalid origin'),
    );
    await controller.passkey();
    expect(controller.snapshot().stage).toBe('closed');
    await controller.password('email', 'password');
    expect(gateway.password).not.toHaveBeenCalled();
  });
  it('does not resurrect an interaction after leaving the document', async () => {
    const { gateway, controller, navigate } = setup();
    let resolveLoad: (value: HostedInteraction) => void = () => {};
    gateway.load.mockReturnValue(
      new Promise((resolve) => {
        resolveLoad = resolve;
      }),
    );
    const pending = controller.start('authorization');
    controller.dispose();
    resolveLoad(interaction);
    await pending;
    await controller.consent('deny');
    expect(controller.snapshot().stage).toBe('closed');
    expect(controller.snapshot().interaction).toBeNull();
    expect(gateway.status).not.toHaveBeenCalled();
    expect(navigate).not.toHaveBeenCalled();
  });

  it('does not navigate when consent finishes after the page was left', async () => {
    const { gateway, controller, navigate } = setup();
    gateway.status.mockResolvedValue(satisfied);
    gateway.load.mockResolvedValue(loggedIn);
    await controller.start('authorization');
    let resolveConsent: (value: string) => void = () => {};
    gateway.consent.mockReturnValue(
      new Promise((resolve) => {
        resolveConsent = resolve;
      }),
    );
    const pending = controller.consent('approve');
    controller.dispose();
    resolveConsent('https://account.example/callback?code=opaque');
    await pending;
    expect(navigate).not.toHaveBeenCalled();
    expect(controller.snapshot().stage).toBe('closed');
  });
  it('loads PAR once and serializes password submissions with rotated CSRF', async () => {
    const { gateway, controller } = setup();
    await Promise.all([
      controller.start('https://identity.example/oauth/authorize'),
      controller.start('ignored'),
    ]);
    expect(gateway.load).toHaveBeenCalledTimes(1);
    gateway.status.mockResolvedValue(stepUp);
    await Promise.all([
      controller.password('user@example.invalid', 'secret'),
      controller.password('other', 'secret'),
    ]);
    expect(gateway.password).toHaveBeenCalledTimes(1);
    expect(gateway.status).toHaveBeenLastCalledWith(loggedIn);
    expect(controller.snapshot().stage).toBe('step-up');
    await controller.consent('approve');
    expect(gateway.consent).not.toHaveBeenCalled();
  });

  it('only permits consent after server confirms MFA and uses session CSRF', async () => {
    const { gateway, controller, navigate } = setup();
    await controller.start('authorization');
    gateway.status.mockResolvedValue(stepUp);
    await controller.password('email', 'password');
    gateway.status.mockResolvedValue(satisfied);
    await controller.totp('123456');
    expect(gateway.stepUpTotp).toHaveBeenCalledWith('session-csrf', '123456');
    await Promise.all([controller.consent('approve'), controller.consent('approve')]);
    expect(gateway.consent).toHaveBeenCalledExactlyOnceWith(loggedIn, 'approve');
    expect(navigate).toHaveBeenCalledExactlyOnceWith(
      'https://account.example/callback?code=opaque',
    );
    expect(controller.snapshot().interaction).toBeNull();
    expect(controller.snapshot().stage).toBe('leaving');
  });

  it('never offers TOTP as a way to satisfy the WebAuthn policy', async () => {
    const { gateway, controller } = setup();
    await controller.start('authorization');
    gateway.status.mockResolvedValue({
      ...stepUp,
      minimumAuthentication: 'recent_webauthn',
    });
    await controller.password('email', 'password');
    await controller.totp('123456');
    expect(gateway.stepUpTotp).not.toHaveBeenCalled();
    await controller.passkey();
    expect(gateway.stepUpPasskey).toHaveBeenCalledWith('session-csrf');
    expect(controller.snapshot().stage).toBe('step-up');
  });

  it('closes on an uncertain mutation without replaying or exposing raw errors', async () => {
    const { gateway, controller } = setup();
    await controller.start('authorization');
    gateway.password.mockRejectedValue(new Error('secret token password'));
    await controller.password('email', 'password');
    await controller.password('email', 'password');
    expect(gateway.password).toHaveBeenCalledTimes(1);
    expect(controller.snapshot().stage).toBe('closed');
    expect(controller.snapshot().interaction).toBeNull();
    expect(controller.snapshot().error).not.toContain('secret');
  });

  it('allows denial before authentication and never infers the callback from input', async () => {
    const { gateway, controller, navigate } = setup();
    gateway.consent.mockResolvedValue('https://account.example/callback?error=access_denied');
    await controller.start('authorization');
    await controller.consent('deny');
    expect(gateway.password).not.toHaveBeenCalled();
    expect(gateway.consent).toHaveBeenCalledWith(interaction, 'deny');
    expect(navigate).toHaveBeenCalledWith('https://account.example/callback?error=access_denied');
  });

  it('rechecks policy when consent rejects an expired proof without retrying consent', async () => {
    const { gateway, controller } = setup();
    gateway.status.mockResolvedValue(satisfied);
    gateway.load.mockResolvedValue(loggedIn);
    await controller.start('authorization');
    gateway.consent.mockRejectedValue(new HostedIdentityError(400));
    gateway.status.mockResolvedValue(stepUp);
    await controller.consent('approve');
    expect(controller.snapshot().stage).toBe('step-up');
    expect(gateway.consent).toHaveBeenCalledTimes(1);
  });
});
