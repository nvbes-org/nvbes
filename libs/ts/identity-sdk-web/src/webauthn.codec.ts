import type {
  WebauthnCreationOptionsJSON,
  WebauthnCredentialJSON,
  WebauthnRequestOptionsJSON,
} from '@nvbes/identity-sdk-core/src/types';

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
    )?.map((credential) => ({
      ...credential,
      id: base64ToArrayBuffer(credential.id, 'excludeCredentials[].id'),
      type: credential.type as PublicKeyCredentialType,
      transports: credential.transports as AuthenticatorTransport[] | undefined,
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
    )?.map((credential) => ({
      ...credential,
      id: base64ToArrayBuffer(credential.id, 'allowCredentials[].id'),
      type: credential.type as PublicKeyCredentialType,
      transports: credential.transports as AuthenticatorTransport[] | undefined,
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

function base64ToArrayBuffer(base64: string | undefined, field: string): ArrayBuffer {
  if (!base64) {
    throw new Error(`Invalid WebAuthn payload: missing ${field}`);
  }
  const clean = base64.replace(/-/g, '+').replace(/_/g, '/');
  const binary = atob(clean);
  const buffer = new ArrayBuffer(binary.length);
  const bytes = new Uint8Array(buffer);
  for (let index = 0; index < binary.length; index++) {
    bytes[index] = binary.charCodeAt(index);
  }
  return buffer;
}

function unwrapCreationOptions(
  options: WebauthnCreationOptionsJSON | { publicKey: WebauthnCreationOptionsJSON },
): WebauthnCreationOptionsJSON {
  return 'publicKey' in options ? options.publicKey : options;
}

function unwrapRequestOptions(
  options: WebauthnRequestOptionsJSON | { publicKey: WebauthnRequestOptionsJSON },
): WebauthnRequestOptionsJSON {
  return 'publicKey' in options ? options.publicKey : options;
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
  for (let index = 0; index < bytes.byteLength; index++) {
    binary += String.fromCharCode(bytes[index]);
  }
  return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=/g, '');
}
