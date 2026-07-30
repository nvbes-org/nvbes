import { assertWebAuthnSupported, isConditionalMediationSupported } from './webauthn.support';
import {
  WebauthnBrowserError,
  type WebauthnCreateOptions,
  type WebauthnGetOptions,
  type WebauthnRegistrationKind,
} from './webauthn.types';

const DEFAULT_WEBAUTHN_TIMEOUT_MS = 60_000;

export async function createWebAuthnCredential(
  publicKey: PublicKeyCredentialCreationOptions,
  options: WebauthnCreateOptions = {},
): Promise<PublicKeyCredential> {
  const credentialKind = getCredentialKind(publicKey);
  await assertWebAuthnSupported(credentialKind);
  return executeCredentialRequest(
    (signal) => navigator.credentials.create({ publicKey, signal }),
    credentialKind,
    options,
  );
}

export async function getWebAuthnCredential(
  publicKey: PublicKeyCredentialRequestOptions,
  options: WebauthnGetOptions = {},
): Promise<PublicKeyCredential> {
  await assertWebAuthnSupported('security_key');
  return executeCredentialRequest(
    (signal) =>
      navigator.credentials.get({
        publicKey,
        signal,
        ...(options.mediation ? { mediation: options.mediation } : {}),
      } as CredentialRequestOptions),
    'security_key',
    options,
  );
}

export async function getConditionalWebAuthnCredential(
  publicKey: PublicKeyCredentialRequestOptions,
  options: { signal?: AbortSignal } = {},
): Promise<PublicKeyCredential | null> {
  if (!(await isConditionalMediationSupported())) {
    return null;
  }
  try {
    const credential = await navigator.credentials.get({
      publicKey,
      mediation: 'conditional',
      ...(options.signal ? { signal: options.signal } : {}),
    } as CredentialRequestOptions);
    return isPublicKeyCredential(credential) ? credential : null;
  } catch {
    return null;
  }
}

export function normalizeWebAuthnError(
  error: unknown,
  kind: WebauthnRegistrationKind = 'security_key',
): WebauthnBrowserError {
  if (error instanceof WebauthnBrowserError) {
    return error;
  }
  if (error instanceof DOMException) {
    return new WebauthnBrowserError(
      webauthnDomExceptionCode(error.name),
      webauthnDomExceptionMessage(error, kind),
      error,
    );
  }
  if (error instanceof Error) {
    return new WebauthnBrowserError('webauthn_failed', error.message, error);
  }
  return new WebauthnBrowserError('webauthn_failed', 'WebAuthn failed.', error);
}

async function executeCredentialRequest(
  request: (signal: AbortSignal) => Promise<Credential | null>,
  kind: WebauthnRegistrationKind,
  options: WebauthnCreateOptions,
): Promise<PublicKeyCredential> {
  const abortController = new AbortController();
  const timeoutId = window.setTimeout(
    () => abortController.abort(timeoutWebAuthnError()),
    options.timeoutMs ?? DEFAULT_WEBAUTHN_TIMEOUT_MS,
  );
  const removeSignalListener = linkAbortSignal(options.signal, abortController);

  try {
    const credential = await request(abortController.signal);
    if (!isPublicKeyCredential(credential)) {
      throw new WebauthnBrowserError(
        'webauthn_credential_missing',
        'WebAuthn did not return a public-key credential.',
      );
    }
    return credential;
  } catch (error) {
    throw normalizeWebAuthnError(error, kind);
  } finally {
    removeSignalListener();
    window.clearTimeout(timeoutId);
  }
}

function timeoutWebAuthnError(): WebauthnBrowserError {
  return new WebauthnBrowserError(
    'webauthn_timeout',
    'WebAuthn timed out before the browser returned a credential.',
  );
}

function linkAbortSignal(
  signal: AbortSignal | undefined,
  abortController: AbortController,
): () => void {
  if (!signal) {
    return () => undefined;
  }
  const abort = () => abortController.abort(signal.reason);
  signal.addEventListener('abort', abort, { once: true });
  if (signal.aborted) abort();
  return () => signal.removeEventListener('abort', abort);
}

function isPublicKeyCredential(credential: Credential | null): credential is PublicKeyCredential {
  return credential?.type === 'public-key' && 'rawId' in credential;
}

function getCredentialKind(
  publicKey: PublicKeyCredentialCreationOptions,
): WebauthnRegistrationKind {
  return publicKey.authenticatorSelection?.authenticatorAttachment === 'platform'
    ? 'passkey'
    : 'security_key';
}

function webauthnDomExceptionCode(name: string): string {
  switch (name) {
    case 'AbortError':
      return 'webauthn_timeout';
    case 'ConstraintError':
      return 'webauthn_constraint_error';
    case 'InvalidStateError':
      return 'webauthn_already_registered';
    case 'NotAllowedError':
      return 'webauthn_not_allowed';
    case 'NotSupportedError':
      return 'webauthn_not_supported';
    case 'SecurityError':
      return 'webauthn_security_error';
    default:
      return 'webauthn_failed';
  }
}

function webauthnDomExceptionMessage(error: DOMException, kind: WebauthnRegistrationKind): string {
  switch (error.name) {
    case 'AbortError':
      return kind === 'passkey'
        ? 'Passkey registration timed out. Confirm with Touch ID or retry in a supported browser.'
        : 'Security key registration timed out. Insert and touch your key, or retry in a supported browser.';
    case 'ConstraintError':
      return 'The selected authenticator does not match the requested WebAuthn policy.';
    case 'InvalidStateError':
      return /pending/iu.test(error.message)
        ? 'A WebAuthn request is already open. Finish or cancel the current browser prompt before trying again.'
        : 'This authenticator is already registered for this account.';
    case 'NotAllowedError':
      return kind === 'passkey'
        ? 'Passkey registration was cancelled, blocked, or no compatible passkey provider was available.'
        : 'Security key registration was cancelled, blocked, or no compatible security key was available.';
    case 'NotSupportedError':
      return 'This browser does not support the requested WebAuthn credential type.';
    case 'SecurityError':
      return 'The WebAuthn domain or origin is invalid for this page.';
    default:
      return error.message || 'WebAuthn failed.';
  }
}
