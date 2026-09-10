import { afterEach, beforeEach, describe, expect, it, type Mock, vi } from 'vite-plus/test';

vi.mock('../pow', () => ({
  fetchPowChallenge: vi.fn().mockResolvedValue({ nonce: 'pow-nonce', difficulty: 16 }),
  solvePowChallenge: vi.fn().mockResolvedValue(42),
}));

import { listMfaFactors, MfaError, removeMfaFactor, requestEmailStepUpCode, stepUp } from '../mfa';

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
      installBrowserContext({ cookie: 'authuser=1; csrf_token=browser-secret' });
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
        expect.objectContaining({ credentials: 'omit' }),
      );
      const request = (globalThis.fetch as Mock).mock.calls[0]?.[1] as RequestInit;
      const headers = new Headers(request.headers);
      expect(headers.get('Authorization')).toBe(`Bearer ${mockToken}`);
      expect(headers.has('X-Auth-User')).toBe(false);
      expect(headers.has('X-CSRF-Token')).toBe(false);
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
});
