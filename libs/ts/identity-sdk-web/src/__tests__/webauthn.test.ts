import { beforeAll, describe, expect, it } from 'vitest';
import {
  normalizeWebAuthnError,
  parseCreationOptions,
  parseRequestOptions,
  serializeCredential,
  WebauthnBrowserError,
} from '../webauthn';

beforeAll(async () => {
  // Polyfill atob/btoa for Node.js test environment
  if (typeof globalThis.atob === 'undefined') {
    const { Buffer } = await import('node:buffer');
    Object.defineProperty(globalThis, 'atob', {
      value: (b64: string) => Buffer.from(b64, 'base64').toString('binary'),
      writable: true,
      configurable: true,
    });
    Object.defineProperty(globalThis, 'btoa', {
      value: (str: string) => Buffer.from(str, 'binary').toString('base64'),
      writable: true,
      configurable: true,
    });
  }
});

describe('WebAuthn helpers', () => {
  const mockServerOptions = {
    challenge: 'dGVzdC1jaGFsbGVuZ2U',
    rp: { name: 'nvbes', id: 'nvbes.fr' },
    user: {
      id: 'dXNlci1pZA',
      name: 'test@nvbes.fr',
      displayName: 'Test User',
    },
    pubKeyCredParams: [{ type: 'public-key', alg: -7 }],
    timeout: 60000,
    attestation: 'none' as const,
  };

  it('parseCreationOptions should convert server options for WebAuthn API', () => {
    const parsed = parseCreationOptions(mockServerOptions);
    expect(parsed.challenge).toBeInstanceOf(ArrayBuffer);
    expect(parsed.user.id).toBeInstanceOf(ArrayBuffer);
    expect(parsed.rp.name).toBe('nvbes');
    expect(parsed.pubKeyCredParams).toHaveLength(1);
  });

  it('parseCreationOptions should accept snake_case server fields', () => {
    const parsed = parseCreationOptions({
      challenge: 'dGVzdC1jaGFsbGVuZ2U',
      rp: { name: 'nvbes', id: 'nvbes.fr' },
      user: {
        id: 'dXNlci1pZA',
        name: 'test@nvbes.fr',
        display_name: 'Test User',
      },
      pub_key_cred_params: [{ type: 'public-key', alg: -7 }],
      exclude_credentials: [{ id: 'Y3JlZGVudGlhbC1pZA', type: 'public-key' }],
      authenticator_selection: { userVerification: 'preferred' },
    } as never);

    expect(parsed.challenge).toBeInstanceOf(ArrayBuffer);
    expect(parsed.user.id).toBeInstanceOf(ArrayBuffer);
    expect(parsed.user.displayName).toBe('Test User');
    expect(parsed.pubKeyCredParams).toHaveLength(1);
    expect(parsed.excludeCredentials).toHaveLength(1);
  });

  it('parseCreationOptions should accept the API publicKey envelope', () => {
    const parsed = parseCreationOptions({
      publicKey: {
        challenge: 'dGVzdC1jaGFsbGVuZ2U',
        rp: { name: 'nvbes', id: 'nvbes.fr' },
        user: {
          id: 'dXNlci1pZA',
          name: 'test@nvbes.fr',
          displayName: 'Test User',
        },
        pubKeyCredParams: [{ type: 'public-key', alg: -7 }],
        authenticatorSelection: {
          authenticatorAttachment: 'cross-platform',
          residentKey: 'discouraged',
          userVerification: 'preferred',
        },
      },
    } as never);

    expect(parsed.challenge).toBeInstanceOf(ArrayBuffer);
    expect(parsed.user.id).toBeInstanceOf(ArrayBuffer);
    expect(parsed.user.displayName).toBe('Test User');
    expect(parsed.authenticatorSelection?.authenticatorAttachment).toBe('cross-platform');
  });

  it('parseRequestOptions should convert server auth options for WebAuthn API', () => {
    const mockAuthOptions = {
      challenge: 'dGVzdC1jaGFsbGVuZ2U',
      timeout: 60000,
      rpId: 'nvbes.fr',
      allowCredentials: [
        {
          id: 'Y3JlZGVudGlhbC1pZA',
          type: 'public-key',
          transports: ['internal' as const],
        },
      ],
      userVerification: 'required' as const,
    };

    const parsed = parseRequestOptions(mockAuthOptions);
    expect(parsed.challenge).toBeInstanceOf(ArrayBuffer);
    expect(parsed.allowCredentials).toHaveLength(1);
    if (parsed.allowCredentials) {
      expect(parsed.allowCredentials[0].id).toBeInstanceOf(ArrayBuffer);
    }
  });

  it('serializeCredential should convert WebAuthn response to JSON', () => {
    const mockCredential = {
      id: 'cred-1',
      rawId: new Uint8Array([1, 2, 3]).buffer,
      type: 'public-key',
      authenticatorAttachment: null,
      response: {
        clientDataJSON: new Uint8Array([4, 5, 6]).buffer,
        attestationObject: new Uint8Array([7, 8, 9]).buffer,
      } as AuthenticatorAttestationResponse,
      getClientExtensionResults: () => ({}),
      toJSON: () => ({}),
    } as unknown as PublicKeyCredential;

    const serialized = serializeCredential(mockCredential);
    expect(serialized.id).toBe('cred-1');
    expect(serialized.type).toBe('public-key');
    expect(typeof serialized.rawId).toBe('string');
    expect(typeof serialized.response.clientDataJSON).toBe('string');
    expect(typeof serialized.response.attestationObject).toBe('string');
  });

  it('serializeCredential should handle missing fields gracefully', () => {
    const mockCredential = {
      id: 'cred-2',
      rawId: new Uint8Array([1, 2, 3]).buffer,
      type: 'public-key',
      authenticatorAttachment: null,
      response: {
        clientDataJSON: new Uint8Array([4, 5, 6]).buffer,
      } as AuthenticatorAttestationResponse,
      getClientExtensionResults: () => ({}),
      toJSON: () => ({}),
    } as unknown as PublicKeyCredential;

    const serialized = serializeCredential(mockCredential);
    expect(serialized.id).toBe('cred-2');
    expect(serialized.response.attestationObject).toBeUndefined();
    expect(serialized.response.signature).toBeUndefined();
    expect(serialized.response.userHandle).toBeUndefined();
  });

  it('normalizeWebAuthnError should map browser timeout errors', () => {
    const error = normalizeWebAuthnError(new DOMException('Timed out', 'AbortError'));

    expect(error).toBeInstanceOf(WebauthnBrowserError);
    expect(error.code).toBe('webauthn_timeout');
    expect(error.message).toContain('timed out');
  });

  it('normalizeWebAuthnError should map duplicate authenticator errors', () => {
    const error = normalizeWebAuthnError(
      new DOMException('Already registered', 'InvalidStateError'),
    );

    expect(error.code).toBe('webauthn_already_registered');
    expect(error.message).toContain('already registered');
  });

  it('normalizeWebAuthnError should explain overlapping browser requests', () => {
    const error = normalizeWebAuthnError(
      new DOMException('A request is already pending.', 'InvalidStateError'),
    );

    expect(error.code).toBe('webauthn_already_registered');
    expect(error.message).toContain('already open');
  });
});
