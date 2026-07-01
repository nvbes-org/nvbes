import type {
  WebauthnCreationOptionsJSON,
  WebauthnCredentialJSON,
  WebauthnRequestOptionsJSON,
} from '@nvbes/identity-sdk-core/src/types';

export type WebauthnRegistrationKind = 'passkey' | 'security_key';

export interface WebauthnSupportReport {
  supported: boolean;
  has_public_key_credential: boolean;
  has_credentials_api: boolean;
  is_secure_context: boolean;
  is_iframe: boolean;
  is_likely_embedded_runtime: boolean;
  platform_authenticator_available: boolean | null;
  conditional_mediation_available: boolean | null;
  reasons: WebauthnUnsupportedReason[];
}

export type WebauthnUnsupportedReason =
  | 'missing_window'
  | 'missing_public_key_credential'
  | 'missing_credentials_api'
  | 'insecure_context'
  | 'embedded_runtime';

export interface WebauthnCreateOptions {
  timeoutMs?: number;
  signal?: AbortSignal;
}

export class WebauthnBrowserError extends Error {
  constructor(
    public readonly code: string,
    message: string,
    public readonly cause?: unknown,
  ) {
    super(message);
    this.name = 'WebauthnBrowserError';
  }
}

export function parseCreationOptions(
  options: WebauthnCreationOptionsJSON | { publicKey: WebauthnCreationOptionsJSON },
): PublicKeyCredentialCreationOptions {
  const publicKey = unwrapCreationOptions(options);
  const normalizedOptions = publicKey as WebauthnCreationOptionsJSON & {
    pub_key_cred_params?: WebauthnCreationOptionsJSON['pubKeyCredParams'];
    exclude_credentials?: WebauthnCreationOptionsJSON['excludeCredentials'];
    authenticator_selection?: WebauthnCreationOptionsJSON['authenticatorSelection'];
  };
  const normalizedUser = publicKey.user as WebauthnCreationOptionsJSON['user'] & {
    display_name?: string;
  };

  return {
    ...(publicKey as unknown as PublicKeyCredentialCreationOptions),
    challenge: base64ToArrayBuffer(publicKey.challenge, 'challenge'),
    user: {
      ...publicKey.user,
      id: base64ToArrayBuffer(publicKey.user.id, 'user.id'),
      displayName: normalizedUser.displayName ?? normalizedUser.display_name,
    },
    pubKeyCredParams: (
      normalizedOptions.pubKeyCredParams ??
      normalizedOptions.pub_key_cred_params ??
      []
    ).map((param) => ({
      ...param,
      type: param.type as PublicKeyCredentialType,
    })),
    excludeCredentials: (
      normalizedOptions.excludeCredentials ?? normalizedOptions.exclude_credentials
    )?.map((cred) => ({
      ...cred,
      id: base64ToArrayBuffer(cred.id, 'excludeCredentials[].id'),
      type: cred.type as PublicKeyCredentialType,
      transports: cred.transports as AuthenticatorTransport[] | undefined,
    })),
    authenticatorSelection: normalizeAuthenticatorSelection(
      normalizedOptions.authenticatorSelection ?? normalizedOptions.authenticator_selection,
    ),
  };
}

export function parseRequestOptions(
  options: WebauthnRequestOptionsJSON | { publicKey: WebauthnRequestOptionsJSON },
): PublicKeyCredentialRequestOptions {
  const publicKey = unwrapRequestOptions(options);
  const normalizedOptions = publicKey as WebauthnRequestOptionsJSON & {
    allow_credentials?: WebauthnRequestOptionsJSON['allowCredentials'];
    rp_id?: string;
    user_verification?: string;
  };

  return {
    ...(publicKey as unknown as PublicKeyCredentialRequestOptions),
    challenge: base64ToArrayBuffer(publicKey.challenge, 'challenge'),
    allowCredentials: (
      normalizedOptions.allowCredentials ?? normalizedOptions.allow_credentials
    )?.map((cred) => ({
      ...cred,
      id: base64ToArrayBuffer(cred.id, 'allowCredentials[].id'),
      type: cred.type as PublicKeyCredentialType,
      transports: cred.transports as AuthenticatorTransport[] | undefined,
    })),
    rpId: normalizedOptions.rpId ?? normalizedOptions.rp_id,
    userVerification: (normalizedOptions.userVerification ??
      normalizedOptions.user_verification) as UserVerificationRequirement | undefined,
  };
}

export function serializeCredential(credential: PublicKeyCredential): WebauthnCredentialJSON {
  const authResponse = credential.response as AuthenticatorAssertionResponse &
    AuthenticatorAttestationResponse;

  const response: WebauthnCredentialJSON = {
    id: credential.id,
    rawId: arrayBufferToBase64(credential.rawId),
    type: credential.type,
    response: {
      clientDataJSON: arrayBufferToBase64(authResponse.clientDataJSON),
    },
  };

  if (authResponse.attestationObject) {
    response.response.attestationObject = arrayBufferToBase64(authResponse.attestationObject);
  }

  if (authResponse.authenticatorData) {
    response.response.authenticatorData = arrayBufferToBase64(authResponse.authenticatorData);
  }

  if (authResponse.signature) {
    response.response.signature = arrayBufferToBase64(authResponse.signature);
  }

  if (authResponse.userHandle) {
    response.response.userHandle = arrayBufferToBase64(authResponse.userHandle);
  }

  return response;
}

export async function getWebAuthnSupport(): Promise<WebauthnSupportReport> {
  const hasWindow = typeof window !== 'undefined';
  const hasPublicKeyCredential = hasWindow && 'PublicKeyCredential' in window;
  const hasCredentialsApi =
    hasWindow && 'navigator' in window && typeof navigator.credentials?.create === 'function';
  const isSecureContext = hasWindow && window.isSecureContext;
  const isIframe = hasWindow ? window.self !== window.top : false;
  const isLikelyEmbeddedRuntime = hasWindow ? isEmbeddedRuntime(navigator.userAgent) : false;
  const reasons: WebauthnUnsupportedReason[] = [];

  if (!hasWindow) {
    reasons.push('missing_window');
  }
  if (!hasPublicKeyCredential) {
    reasons.push('missing_public_key_credential');
  }
  if (!hasCredentialsApi) {
    reasons.push('missing_credentials_api');
  }
  if (!isSecureContext) {
    reasons.push('insecure_context');
  }
  if (isLikelyEmbeddedRuntime) {
    reasons.push('embedded_runtime');
  }

  return {
    supported:
      hasWindow &&
      hasPublicKeyCredential &&
      hasCredentialsApi &&
      isSecureContext &&
      reasons.length === 0,
    has_public_key_credential: hasPublicKeyCredential,
    has_credentials_api: hasCredentialsApi,
    is_secure_context: isSecureContext,
    is_iframe: isIframe,
    is_likely_embedded_runtime: isLikelyEmbeddedRuntime,
    platform_authenticator_available: await getPlatformAuthenticatorAvailability(),
    conditional_mediation_available: await getConditionalMediationAvailability(),
    reasons,
  };
}

export async function assertWebAuthnSupported(kind: WebauthnRegistrationKind): Promise<void> {
  const support = await getWebAuthnSupport();
  if (!support.supported) {
    throw new WebauthnBrowserError(
      'webauthn_unsupported',
      unsupportedWebAuthnMessage(support, kind),
    );
  }
}

export async function createWebAuthnCredential(
  publicKey: PublicKeyCredentialCreationOptions,
  options: WebauthnCreateOptions = {},
): Promise<PublicKeyCredential> {
  await assertWebAuthnSupported(getCredentialKind(publicKey));

  const abortController = new AbortController();
  const timeoutId = window.setTimeout(
    () => abortController.abort(timeoutWebAuthnError()),
    options.timeoutMs ?? 60_000,
  );
  const removeSignalListener = linkAbortSignal(options.signal, abortController);

  try {
    const credential = await navigator.credentials.create({
      publicKey,
      signal: abortController.signal,
    });

    if (!isPublicKeyCredential(credential)) {
      throw new WebauthnBrowserError(
        'webauthn_credential_missing',
        'WebAuthn did not return a public-key credential.',
      );
    }

    return credential;
  } catch (error) {
    throw normalizeWebAuthnError(error, getCredentialKind(publicKey));
  } finally {
    removeSignalListener();
    window.clearTimeout(timeoutId);
  }
}

export interface WebauthnGetOptions extends WebauthnCreateOptions {
  mediation?: CredentialMediationRequirement;
}

export async function getWebAuthnCredential(
  publicKey: PublicKeyCredentialRequestOptions,
  options: WebauthnGetOptions = {},
): Promise<PublicKeyCredential> {
  await assertWebAuthnSupported('security_key');

  const abortController = new AbortController();
  const timeoutId = window.setTimeout(
    () => abortController.abort(timeoutWebAuthnError()),
    options.timeoutMs ?? 60_000,
  );
  const removeSignalListener = linkAbortSignal(options.signal, abortController);

  try {
    const credential = await navigator.credentials.get({
      publicKey,
      signal: abortController.signal,
      ...(options.mediation ? { mediation: options.mediation } : {}),
    } as CredentialRequestOptions);

    if (!isPublicKeyCredential(credential)) {
      throw new WebauthnBrowserError(
        'webauthn_credential_missing',
        'WebAuthn did not return a public-key credential.',
      );
    }

    return credential;
  } catch (error) {
    throw normalizeWebAuthnError(error, 'security_key');
  } finally {
    removeSignalListener();
    window.clearTimeout(timeoutId);
  }
}

export async function getConditionalWebAuthnCredential(
  publicKey: PublicKeyCredentialRequestOptions,
): Promise<PublicKeyCredential | null> {
  if (!(await isConditionalMediationSupported())) {
    return null;
  }
  try {
    const credential = await navigator.credentials.get({
      publicKey,
      mediation: 'conditional',
    } as CredentialRequestOptions);
    if (!credential || !isPublicKeyCredential(credential)) {
      return null;
    }
    return credential;
  } catch {
    return null;
  }
}

export async function isConditionalMediationSupported(): Promise<boolean> {
  if (
    typeof PublicKeyCredential === 'undefined' ||
    typeof PublicKeyCredential.isConditionalMediationAvailable !== 'function'
  ) {
    return false;
  }
  try {
    return await PublicKeyCredential.isConditionalMediationAvailable();
  } catch {
    return false;
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

function base64ToArrayBuffer(base64: string | undefined, field: string): ArrayBuffer {
  if (!base64) {
    throw new Error(`Invalid WebAuthn payload: missing ${field}`);
  }
  const clean = base64.replace(/-/g, '+').replace(/_/g, '/');
  const binary = atob(clean);
  const buffer = new ArrayBuffer(binary.length);
  const bytes = new Uint8Array(buffer);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }
  return buffer;
}

async function getPlatformAuthenticatorAvailability(): Promise<boolean | null> {
  if (
    typeof PublicKeyCredential === 'undefined' ||
    typeof PublicKeyCredential.isUserVerifyingPlatformAuthenticatorAvailable !== 'function'
  ) {
    return null;
  }

  try {
    return await PublicKeyCredential.isUserVerifyingPlatformAuthenticatorAvailable();
  } catch {
    return null;
  }
}

async function getConditionalMediationAvailability(): Promise<boolean | null> {
  if (
    typeof PublicKeyCredential === 'undefined' ||
    typeof PublicKeyCredential.isConditionalMediationAvailable !== 'function'
  ) {
    return null;
  }

  try {
    return await PublicKeyCredential.isConditionalMediationAvailable();
  } catch {
    return null;
  }
}

function isEmbeddedRuntime(userAgent: string): boolean {
  return (
    userAgent.includes('Electron') || userAgent.includes(' Code/') || userAgent.includes('VSCode')
  );
}

function unsupportedWebAuthnMessage(
  support: WebauthnSupportReport,
  kind: WebauthnRegistrationKind,
): string {
  if (support.reasons.includes('insecure_context')) {
    return 'WebAuthn requires HTTPS or localhost.';
  }
  if (
    support.reasons.includes('missing_public_key_credential') ||
    support.reasons.includes('missing_credentials_api')
  ) {
    return 'This browser does not support WebAuthn.';
  }
  if (support.reasons.includes('embedded_runtime')) {
    return embeddedRuntimeMessage(kind);
  }
  return 'WebAuthn is not available in this browser context.';
}

function embeddedRuntimeMessage(kind: WebauthnRegistrationKind): string {
  if (kind === 'passkey') {
    return 'Passkeys are unreliable in the VS Code/Electron embedded browser. Open this page in Chrome, Arc, Safari, or Edge.';
  }

  return 'Security keys are unreliable in the VS Code/Electron embedded browser. Open this page in Chrome, Arc, Safari, or Edge.';
}

function timeoutWebAuthnError(): WebauthnBrowserError {
  return new WebauthnBrowserError(
    'webauthn_timeout',
    'WebAuthn timed out before the browser returned a credential.',
  );
}

function linkAbortSignal(signal: AbortSignal | undefined, abortController: AbortController) {
  if (!signal) {
    return () => undefined;
  }

  const abort = () => abortController.abort(signal.reason);
  signal.addEventListener('abort', abort, { once: true });
  if (signal.aborted) {
    abort();
  }

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
      if (/pending/iu.test(error.message)) {
        return 'A WebAuthn request is already open. Finish or cancel the current browser prompt before trying again.';
      }
      return 'This authenticator is already registered for this account.';
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

function unwrapCreationOptions(
  options: WebauthnCreationOptionsJSON | { publicKey: WebauthnCreationOptionsJSON },
): WebauthnCreationOptionsJSON {
  if ('publicKey' in options) {
    return options.publicKey;
  }
  return options;
}

function unwrapRequestOptions(
  options: WebauthnRequestOptionsJSON | { publicKey: WebauthnRequestOptionsJSON },
): WebauthnRequestOptionsJSON {
  if ('publicKey' in options) {
    return options.publicKey;
  }
  return options;
}

function normalizeAuthenticatorSelection(
  selection:
    | WebauthnCreationOptionsJSON['authenticatorSelection']
    | {
        authenticatorAttachment?: string;
        requireResidentKey?: boolean;
        residentKey?: string;
        userVerification?: string;
      }
    | undefined,
): AuthenticatorSelectionCriteria | undefined {
  if (!selection) {
    return undefined;
  }

  return {
    ...selection,
    authenticatorAttachment: selection.authenticatorAttachment as
      | AuthenticatorAttachment
      | undefined,
    residentKey: selection.residentKey as ResidentKeyRequirement | undefined,
    userVerification: selection.userVerification as UserVerificationRequirement | undefined,
  };
}

function arrayBufferToBase64(buffer: ArrayBuffer): string {
  const bytes = new Uint8Array(buffer);
  let binary = '';
  for (let i = 0; i < bytes.byteLength; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=/g, '');
}
