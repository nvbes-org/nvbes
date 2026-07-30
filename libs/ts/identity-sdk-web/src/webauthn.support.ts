import {
  WebauthnBrowserError,
  type WebauthnRegistrationKind,
  type WebauthnSupportReport,
  type WebauthnUnsupportedReason,
} from './webauthn.types';

export async function getWebAuthnSupport(): Promise<WebauthnSupportReport> {
  const hasWindow = typeof window !== 'undefined';
  const hasPublicKeyCredential = hasWindow && 'PublicKeyCredential' in window;
  const hasCredentialsApi =
    hasWindow && 'navigator' in window && typeof navigator.credentials?.create === 'function';
  const isSecureContext = hasWindow && window.isSecureContext;
  const isIframe = hasWindow ? window.self !== window.top : false;
  const isLikelyEmbeddedRuntime = hasWindow ? isEmbeddedRuntime(navigator.userAgent) : false;
  const reasons: WebauthnUnsupportedReason[] = [];

  if (!hasWindow) reasons.push('missing_window');
  if (!hasPublicKeyCredential) reasons.push('missing_public_key_credential');
  if (!hasCredentialsApi) reasons.push('missing_credentials_api');
  if (!isSecureContext) reasons.push('insecure_context');
  if (isLikelyEmbeddedRuntime) reasons.push('embedded_runtime');

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
