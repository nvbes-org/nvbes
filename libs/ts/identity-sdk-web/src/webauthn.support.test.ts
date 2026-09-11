import { afterEach, describe, expect, it, vi } from 'vite-plus/test';
import {
  assertWebAuthnSupported,
  getWebAuthnSupport,
  isConditionalMediationSupported,
} from './webauthn.support';

function browser(userAgent = 'Mozilla/5.0', secure = true) {
  const credential = {
    isConditionalMediationAvailable: vi.fn().mockResolvedValue(true),
    isUserVerifyingPlatformAuthenticatorAvailable: vi.fn().mockResolvedValue(true),
  };
  const navigator = { userAgent, credentials: { create: vi.fn() } };
  const top = {};
  vi.stubGlobal('navigator', navigator);
  vi.stubGlobal('PublicKeyCredential', credential);
  vi.stubGlobal('window', {
    navigator,
    isSecureContext: secure,
    self: top,
    top,
    PublicKeyCredential: credential,
  });
  return credential;
}

afterEach(() => vi.unstubAllGlobals());

describe('WebAuthn capability detection', () => {
  it('reports a secure capable browser and its optional capabilities', async () => {
    browser();
    await expect(getWebAuthnSupport()).resolves.toEqual({
      supported: true,
      has_public_key_credential: true,
      has_credentials_api: true,
      is_secure_context: true,
      is_iframe: false,
      is_likely_embedded_runtime: false,
      platform_authenticator_available: true,
      conditional_mediation_available: true,
      reasons: [],
    });
    await expect(assertWebAuthnSupported('passkey')).resolves.toBeUndefined();
    await expect(isConditionalMediationSupported()).resolves.toBe(true);
  });

  it('rejects insecure contexts with an actionable message', async () => {
    browser('Mozilla/5.0', false);
    expect((await getWebAuthnSupport()).reasons).toEqual(['insecure_context']);
    await expect(assertWebAuthnSupported('passkey')).rejects.toThrow(
      'WebAuthn requires HTTPS or localhost.',
    );
  });

  it.each(['Electron', 'Mozilla Code/1.0', 'VSCode'])(
    'identifies embedded runtime %s',
    async (userAgent) => {
      browser(userAgent);
      const report = await getWebAuthnSupport();
      expect(report.supported).toBe(false);
      expect(report.is_likely_embedded_runtime).toBe(true);
      expect(report.reasons).toEqual(['embedded_runtime']);
      await expect(assertWebAuthnSupported('passkey')).rejects.toThrow('Passkeys are unreliable');
      await expect(assertWebAuthnSupported('security_key')).rejects.toThrow(
        'Security keys are unreliable',
      );
    },
  );

  it('handles missing browser APIs without calling them', async () => {
    vi.stubGlobal('window', undefined);
    vi.stubGlobal('PublicKeyCredential', undefined);
    const report = await getWebAuthnSupport();
    expect(report.supported).toBe(false);
    expect(report.reasons).toEqual([
      'missing_window',
      'missing_public_key_credential',
      'missing_credentials_api',
      'insecure_context',
    ]);
    expect(report.platform_authenticator_available).toBeNull();
    expect(report.conditional_mediation_available).toBeNull();
    await expect(isConditionalMediationSupported()).resolves.toBe(false);
  });

  it('distinguishes unavailable optional APIs from a negative capability result', async () => {
    const credential = browser();
    credential.isConditionalMediationAvailable.mockResolvedValue(false);
    credential.isUserVerifyingPlatformAuthenticatorAvailable.mockResolvedValue(false);
    expect((await getWebAuthnSupport()).platform_authenticator_available).toBe(false);
    expect((await getWebAuthnSupport()).conditional_mediation_available).toBe(false);
    vi.stubGlobal('PublicKeyCredential', {});
    expect((await getWebAuthnSupport()).platform_authenticator_available).toBeNull();
    await expect(isConditionalMediationSupported()).resolves.toBe(false);
  });

  it('contains rejected capability probes', async () => {
    const credential = browser();
    credential.isConditionalMediationAvailable.mockRejectedValue(new Error('blocked'));
    credential.isUserVerifyingPlatformAuthenticatorAvailable.mockRejectedValue(
      new Error('blocked'),
    );
    const report = await getWebAuthnSupport();
    expect(report.supported).toBe(true);
    expect(report.platform_authenticator_available).toBeNull();
    expect(report.conditional_mediation_available).toBeNull();
    await expect(isConditionalMediationSupported()).resolves.toBe(false);
  });

  it('reports iframe context and missing required credential API', async () => {
    browser();
    vi.stubGlobal('window', { navigator, isSecureContext: true, self: {}, top: {} });
    expect((await getWebAuthnSupport()).is_iframe).toBe(true);
    await expect(assertWebAuthnSupported('passkey')).rejects.toThrow(
      'This browser does not support WebAuthn.',
    );
    vi.stubGlobal('navigator', { userAgent: 'Mozilla', credentials: {} });
    expect((await getWebAuthnSupport()).has_credentials_api).toBe(false);
  });
});
