import { afterEach, beforeEach, describe, expect, it, type Mock, vi } from 'vite-plus/test';

vi.mock('../pow', () => ({
  fetchPowChallenge: vi.fn().mockResolvedValue({ nonce: 'pow-nonce', difficulty: 16 }),
  solvePowChallenge: vi.fn().mockResolvedValue(42),
}));

import {
  confirmTotp,
  generateRecoveryCodes,
  listMfaFactors,
  MfaError,
  removeMfaFactor,
  requestEmailStepUpCode,
  setupTotp,
  startWebAuthnAuthentication,
  stepUp,
} from '../mfa';

const mockToken = 'test-jwt-token';
const mockBaseUrl = 'https://account.nvbes.fr';

function installBrowserContext({
  cookie,
  pathname = '/',
  search = '',
}: {
  cookie: string;
  pathname?: string;
  search?: string;
}) {
  Object.defineProperty(globalThis, 'window', {
    configurable: true,
    value: {
      location: { pathname, search },
    },
  });
  Object.defineProperty(globalThis, 'document', {
    configurable: true,
    value: { cookie },
  });
}

describe('MFA API functions', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  afterEach(() => {
    Reflect.deleteProperty(globalThis, 'window');
    Reflect.deleteProperty(globalThis, 'document');
  });

  describe('listMfaFactors', () => {
    it('should return factors on success', async () => {
      const mockFactors = {
        factors: [
          {
            id: 'factor-1',
            factor_type: 'totp',
            status: 'active',
            label: 'My TOTP',
          },
        ],
        mfa_enabled: true,
      };

      globalThis.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: () => Promise.resolve(mockFactors),
      });

      const result = await listMfaFactors(mockBaseUrl, mockToken);
      expect(result.mfa_enabled).toBe(true);
      expect(result.factors).toHaveLength(1);
      expect(result.factors[0].factor_type).toBe('totp');
    });

    it('passes pagination parameters to the factors endpoint', async () => {
      globalThis.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: () =>
          Promise.resolve({
            factors: [],
            mfa_enabled: true,
            next_cursor: null,
            has_more: false,
          }),
      });

      await listMfaFactors(mockBaseUrl, mockToken, {
        limit: 25,
        cursor: 'opaque-cursor',
      });

      expect(globalThis.fetch).toHaveBeenCalledWith(
        `${mockBaseUrl}/auth/mfa/factors?limit=25&cursor=opaque-cursor`,
        expect.objectContaining({ credentials: 'include' }),
      );
    });

    it('should work with cookie auth when token is absent', async () => {
      const mockFactors = { factors: [], mfa_enabled: false };

      globalThis.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: () => Promise.resolve(mockFactors),
      });

      const result = await listMfaFactors(mockBaseUrl);
      expect(result.mfa_enabled).toBe(false);

      const callHeaders = (globalThis.fetch as Mock).mock.calls[0][1].headers as Record<
        string,
        string
      >;
      expect(callHeaders.Authorization).toBeUndefined();
    });

    it('should throw MfaError on failure', async () => {
      globalThis.fetch = vi.fn().mockResolvedValue({
        ok: false,
        status: 401,
        statusText: 'Unauthorized',
        json: () =>
          Promise.resolve({
            error: { code: 'step_up_required', message: '...' },
          }),
      });

      await expect(listMfaFactors(mockBaseUrl, mockToken)).rejects.toThrow(MfaError);
    });
  });

  describe('setupTotp', () => {
    it('should return setup result with secret and provisioning_uri', async () => {
      const mockSetup = {
        factor: { id: 'factor-2', factor_type: 'totp', status: 'pending' },
        secret_base32: 'JBSWY3DPEHPK3PXP',
        provisioning_uri: 'otpauth://totp/nvbes:test@nvbes.fr?secret=JBSWY3DPEHPK3PXP',
      };

      globalThis.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: () => Promise.resolve(mockSetup),
      });

      const result = await setupTotp(mockBaseUrl, 'Phone', mockToken);
      expect(result.secret_base32).toBe('JBSWY3DPEHPK3PXP');
      expect(result.provisioning_uri).toContain('otpauth://totp');
      expect(result.factor.status).toBe('pending');
    });

    it('should work with cookie auth when token is absent', async () => {
      globalThis.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: () => Promise.resolve({}),
      });

      await setupTotp(mockBaseUrl, 'Phone');
      const body = JSON.parse((globalThis.fetch as Mock).mock.calls[0][1].body);
      expect(body.label).toBe('Phone');
    });

    it('should send scoped csrf and authuser headers with cookie auth', async () => {
      installBrowserContext({
        cookie: 'csrf_token=base; csrf_token_1=one',
        search: '?authuser=1',
      });
      globalThis.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: () => Promise.resolve({}),
      });

      await setupTotp(mockBaseUrl, 'Phone');
      const headers = (globalThis.fetch as Mock).mock.calls[0][1].headers as Headers;

      expect(headers.get('X-Auth-User')).toBe('1');
      expect(headers.get('X-CSRF-Token')).toBe('one');
    });

    it('should send label in request body', async () => {
      globalThis.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: () => Promise.resolve({}),
      });

      await setupTotp(mockBaseUrl, 'Work TOTP', mockToken);
      const body = JSON.parse((globalThis.fetch as Mock).mock.calls[0][1].body);
      expect(body.label).toBe('Work TOTP');
    });
  });

  describe('confirmTotp', () => {
    it('should confirm TOTP with factor_id and code', async () => {
      const mockConfirm = {
        factor: { id: 'factor-2', factor_type: 'totp', status: 'active' },
        mfa_enabled: true,
      };

      globalThis.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: () => Promise.resolve(mockConfirm),
      });

      const result = await confirmTotp(mockBaseUrl, 'factor-2', '123456', mockToken);
      expect(result.mfa_enabled).toBe(true);
      expect(result.factor.status).toBe('active');
    });

    it('should work with cookie auth when token is absent', async () => {
      globalThis.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: () => Promise.resolve({}),
      });

      await confirmTotp(mockBaseUrl, 'factor-2', '123456');
    });

    it('should send correct body', async () => {
      globalThis.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: () => Promise.resolve({}),
      });

      await confirmTotp(mockBaseUrl, 'factor-2', '123456', mockToken);
      const body = JSON.parse((globalThis.fetch as Mock).mock.calls[0][1].body);
      expect(body.factor_id).toBe('factor-2');
      expect(body.code).toBe('123456');
    });
  });

  describe('generateRecoveryCodes', () => {
    it('should return recovery codes on success', async () => {
      const mockCodes = { codes: ['code1-abc', 'code2-def', 'code3-ghi'] };

      globalThis.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: () => Promise.resolve(mockCodes),
      });

      const result = await generateRecoveryCodes(mockBaseUrl, 'password123', mockToken);
      expect(result.codes).toHaveLength(3);
    });

    it('should work with cookie auth when token is absent', async () => {
      globalThis.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: () => Promise.resolve({ codes: [] }),
      });

      await generateRecoveryCodes(mockBaseUrl, 'password123');
    });
  });

  describe('removeMfaFactor', () => {
    it('should send DELETE request and resolve on success', async () => {
      globalThis.fetch = vi.fn().mockResolvedValue({
        ok: true,
      });

      await removeMfaFactor(mockBaseUrl, 'factor-1', mockToken);
      expect(globalThis.fetch).toHaveBeenCalledWith(
        `${mockBaseUrl}/auth/mfa/factors/factor-1`,
        expect.objectContaining({ method: 'DELETE' }),
      );
    });

    it('should work with cookie auth when token is absent', async () => {
      globalThis.fetch = vi.fn().mockResolvedValue({
        ok: true,
      });

      await removeMfaFactor(mockBaseUrl, 'factor-1');
    });

    it('should throw MfaError on failure', async () => {
      globalThis.fetch = vi.fn().mockResolvedValue({
        ok: false,
        status: 401,
        statusText: 'Unauthorized',
        json: () =>
          Promise.resolve({
            error: { code: 'step_up_required', message: '...' },
          }),
      });

      await expect(removeMfaFactor(mockBaseUrl, 'invalid-id', mockToken)).rejects.toThrow(MfaError);
    });
  });

  describe('stepUp', () => {
    it('should send password credential and return validUntil', async () => {
      const mockResult = { success: true, valid_until: '2024-12-31T23:59:59Z' };

      globalThis.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: () => Promise.resolve(mockResult),
      });

      const result = await stepUp(mockBaseUrl, { password: 'pass123' }, mockToken);
      expect(result.success).toBe(true);
      expect(result.valid_until).toBe('2024-12-31T23:59:59Z');
      const body = JSON.parse((globalThis.fetch as Mock).mock.calls[0][1].body);
      expect(body.pow_nonce).toBe('pow-nonce');
      expect(body.pow_solution).toBe('42');
    });

    it('should send totp_code when provided', async () => {
      globalThis.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: () => Promise.resolve({ success: true }),
      });

      await stepUp(mockBaseUrl, { totpCode: '123456' });
      const body = JSON.parse((globalThis.fetch as Mock).mock.calls[0][1].body);
      expect(body.totp_code).toBe('123456');
      expect(body.password).toBeNull();
    });

    it('should send webauthn_challenge_id when provided', async () => {
      globalThis.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: () => Promise.resolve({ success: true }),
      });

      await stepUp(mockBaseUrl, {
        webauthnResponse: { id: 'credential-id' },
        webauthnChallengeId: 'challenge-id',
      });
      const body = JSON.parse((globalThis.fetch as Mock).mock.calls[0][1].body);
      expect(body.webauthn_response).toEqual({ id: 'credential-id' });
      expect(body.webauthn_challenge_id).toBe('challenge-id');
    });

    it('binds an email code to the password change purpose', async () => {
      globalThis.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: () => Promise.resolve({ success: true }),
      });

      await stepUp(mockBaseUrl, {
        purpose: 'password_change',
        emailCode: '123456',
        emailChallengeId: 'challenge-id',
      });
      const body = JSON.parse((globalThis.fetch as Mock).mock.calls[0][1].body);
      expect(body.purpose).toBe('password_change');
      expect(body.email_code).toBe('123456');
      expect(body.email_challenge_id).toBe('challenge-id');
      expect(body.password).toBeNull();
    });

    it('should throw MfaError on invalid credential', async () => {
      globalThis.fetch = vi.fn().mockResolvedValue({
        ok: false,
        status: 401,
        statusText: 'Unauthorized',
        json: () =>
          Promise.resolve({
            error: { code: 'invalid_credentials', message: '...' },
          }),
      });

      await expect(stepUp(mockBaseUrl, { password: 'wrong' }, mockToken)).rejects.toThrow(MfaError);
    });
  });

  describe('requestEmailStepUpCode', () => {
    it('requests a code only for password change', async () => {
      globalThis.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: () =>
          Promise.resolve({
            challenge_id: 'challenge-id',
            expires_at: '2026-07-28T12:00:00Z',
          }),
      });

      const result = await requestEmailStepUpCode(mockBaseUrl, 'password_change', mockToken);
      expect(result.challenge_id).toBe('challenge-id');
      expect(globalThis.fetch).toHaveBeenCalledWith(
        `${mockBaseUrl}/auth/step-up/email/request`,
        expect.objectContaining({
          method: 'POST',
          body: JSON.stringify({ purpose: 'password_change' }),
        }),
      );
    });
  });

  describe('startWebAuthnAuthentication', () => {
    it('should preserve the server challenge id', async () => {
      globalThis.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: () =>
          Promise.resolve({
            challenge_id: 'challenge-id',
            options: {
              challenge: 'dGVzdC1jaGFsbGVuZ2U',
              allowCredentials: [{ id: 'Y3JlZGVudGlhbC1pZA', type: 'public-key' }],
            },
          }),
      });

      const result = await startWebAuthnAuthentication(mockBaseUrl);

      expect(result.challengeId).toBe('challenge-id');
      expect(result.options.challenge).toBeInstanceOf(ArrayBuffer);
    });

    it('should reject malformed start responses without a challenge id', async () => {
      globalThis.fetch = vi.fn().mockResolvedValue({
        ok: true,
        status: 200,
        json: () =>
          Promise.resolve({
            options: {
              challenge: 'dGVzdC1jaGFsbGVuZ2U',
            },
          }),
      });

      await expect(startWebAuthnAuthentication(mockBaseUrl)).rejects.toThrow(MfaError);
    });
  });
});
