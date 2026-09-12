import { afterEach, beforeEach, expect, it, vi } from 'vite-plus/test';
import {
  createWebAuthnCredential,
  getWebAuthnCredential,
  getConditionalWebAuthnCredential,
  normalizeWebAuthnError,
} from './webauthn.credentials';
import { WebauthnBrowserError } from './webauthn.types';

const credential = { id: 'key', type: 'public-key', rawId: new ArrayBuffer(4) };
const get = vi.fn();
const create = vi.fn();
const request = { challenge: new Uint8Array([1, 2]) };
const creation: PublicKeyCredentialCreationOptions = {
  ...request,
  rp: { name: 'Nvbes' },
  user: { id: new Uint8Array([3]), name: 'person', displayName: 'Person' },
  pubKeyCredParams: [{ type: 'public-key', alg: -7 }],
};

beforeEach(() => {
  vi.useFakeTimers();
  get.mockReset().mockResolvedValue(credential);
  create.mockReset().mockResolvedValue(credential);
  const navigator = { userAgent: 'Firefox', credentials: { get, create } };
  const publicKey = { isConditionalMediationAvailable: async () => true };
  vi.stubGlobal('navigator', navigator);
  vi.stubGlobal('PublicKeyCredential', publicKey);
  vi.stubGlobal('window', {
    navigator,
    PublicKeyCredential: publicKey,
    isSecureContext: true,
    self: null,
    top: null,
    setTimeout,
    clearTimeout,
  });
});
afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

it.each([undefined, 'platform', 'cross-platform'] as const)(
  'creates credentials for attachment %s and clears the timeout',
  async (attachment) => {
    const publicKey = {
      ...creation,
      ...(attachment ? { authenticatorSelection: { authenticatorAttachment: attachment } } : {}),
    };
    await expect(createWebAuthnCredential(publicKey)).resolves.toBe(credential);
    expect(create).toHaveBeenCalledExactlyOnceWith({ publicKey, signal: expect.any(AbortSignal) });
    expect(vi.getTimerCount()).toBe(0);
  },
);
it.each([undefined, 'required'] as const)(
  'forwards optional mediation %s on authentication',
  async (mediation) => {
    await expect(getWebAuthnCredential(request, { mediation })).resolves.toBe(credential);
    expect(get).toHaveBeenCalledExactlyOnceWith({
      publicKey: request,
      signal: expect.any(AbortSignal),
      ...(mediation ? { mediation } : {}),
    });
    expect(vi.getTimerCount()).toBe(0);
  },
);
it.each([null, { type: 'password' }, { type: 'public-key' }])(
  'rejects a missing or invalid credential %j',
  async (value) => {
    get.mockResolvedValue(value);
    await expect(getWebAuthnCredential(request)).rejects.toMatchObject({
      code: 'webauthn_credential_missing',
    });
    expect(vi.getTimerCount()).toBe(0);
  },
);
it.each([true, false])(
  'propagates caller cancellation (already aborted=%s) and removes its listener',
  async (alreadyAborted) => {
    const controller = new AbortController();
    const remove = vi.spyOn(controller.signal, 'removeEventListener');
    if (alreadyAborted) controller.abort(new Error('cancelled'));
    get.mockImplementation(
      ({ signal }: { signal: AbortSignal }) =>
        new Promise((_resolve, reject) => {
          if (signal.aborted) reject(signal.reason);
          else signal.addEventListener('abort', () => reject(signal.reason));
        }),
    );
    const pending = expect(
      getWebAuthnCredential(request, { signal: controller.signal }),
    ).rejects.toThrow('cancelled');
    if (!alreadyAborted) {
      await vi.advanceTimersByTimeAsync(0);
      controller.abort(new Error('cancelled'));
    }
    await pending;
    expect(remove).toHaveBeenCalledWith('abort', expect.any(Function));
    expect(vi.getTimerCount()).toBe(0);
  },
);
it.each([undefined, 20])('applies timeout %s and releases its timer', async (timeoutMs) => {
  get.mockImplementation(
    ({ signal }: { signal: AbortSignal }) =>
      new Promise((_resolve, reject) => {
        signal.addEventListener('abort', () => reject(signal.reason));
      }),
  );
  const result = expect(getWebAuthnCredential(request, { timeoutMs })).rejects.toMatchObject({
    code: 'webauthn_timeout',
  });
  await vi.advanceTimersByTimeAsync(timeoutMs ?? 60000);
  await result;
  expect(vi.getTimerCount()).toBe(0);
});
it('forwards conditional mediation and treats cancellation as absence', async () => {
  const signal = new AbortController().signal;
  await expect(getConditionalWebAuthnCredential(request, { signal })).resolves.toBe(credential);
  expect(get).toHaveBeenLastCalledWith({ publicKey: request, mediation: 'conditional', signal });
  get.mockResolvedValue(null);
  await expect(getConditionalWebAuthnCredential(request)).resolves.toBeNull();
  get.mockRejectedValue(new Error('cancelled'));
  await expect(getConditionalWebAuthnCredential(request)).resolves.toBeNull();
  vi.stubGlobal('PublicKeyCredential', undefined);
  get.mockClear();
  await expect(getConditionalWebAuthnCredential(request)).resolves.toBeNull();
  expect(get).not.toHaveBeenCalled();
});
it.each([
  ['AbortError', 'webauthn_timeout'],
  ['ConstraintError', 'webauthn_constraint_error'],
  ['InvalidStateError', 'webauthn_already_registered'],
  ['NotAllowedError', 'webauthn_not_allowed'],
  ['NotSupportedError', 'webauthn_not_supported'],
  ['SecurityError', 'webauthn_security_error'],
  ['UnknownError', 'webauthn_failed'],
])('normalizes %s without losing the original cause', (name, code) => {
  const cause = new DOMException('Browser failure', name);
  for (const kind of ['passkey', 'security_key'] as const) {
    const error = normalizeWebAuthnError(cause, kind);
    expect(error.code).toBe(code);
    expect(error.message.length).toBeGreaterThan(10);
  }
});
it('distinguishes pending requests, existing credentials and unknown failures', () => {
  expect(
    normalizeWebAuthnError(new DOMException('PENDING', 'InvalidStateError')).message,
  ).toContain('already open');
  expect(normalizeWebAuthnError(new DOMException('', 'InvalidStateError')).message).toContain(
    'already registered',
  );
  expect(normalizeWebAuthnError(new DOMException('', 'UnknownError')).message).toBe(
    'WebAuthn failed.',
  );
  expect(normalizeWebAuthnError('unknown').message).toBe('WebAuthn failed.');
  const known = new WebauthnBrowserError('test', 'Known failure');
  expect(normalizeWebAuthnError(known)).toBe(known);
});
