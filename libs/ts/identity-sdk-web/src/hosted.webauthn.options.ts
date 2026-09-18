import type {
  WebauthnCreationOptionsJSON,
  WebauthnRequestOptionsJSON,
} from '@nvbes/identity-sdk-core/src/types';
import { parseCreationOptions, parseRequestOptions } from './webauthn.codec';
import { record } from './hosted.transport';

function descriptors(value: unknown): boolean {
  return (
    value === undefined ||
    (Array.isArray(value) &&
      value.every((item: unknown) => {
        const descriptor = record(item);
        return (
          typeof descriptor.id === 'string' &&
          descriptor.type === 'public-key' &&
          (descriptor.transports === undefined ||
            (Array.isArray(descriptor.transports) &&
              descriptor.transports.every((transport: unknown) => typeof transport === 'string')))
        );
      }))
  );
}

function isCreation(
  value: Record<string, unknown>,
): value is Record<string, unknown> & WebauthnCreationOptionsJSON {
  const user = record(value.user);
  const rp = record(value.rp);
  const selection = record(value.authenticatorSelection);
  return (
    typeof value.challenge === 'string' &&
    typeof user.id === 'string' &&
    typeof user.name === 'string' &&
    typeof user.displayName === 'string' &&
    typeof rp.name === 'string' &&
    rp.id === location.hostname &&
    Array.isArray(value.pubKeyCredParams) &&
    value.pubKeyCredParams.length > 0 &&
    value.pubKeyCredParams.every((item: unknown) => {
      const param = record(item);
      return (
        param.type === 'public-key' &&
        typeof param.alg === 'number' &&
        Number.isSafeInteger(param.alg)
      );
    }) &&
    descriptors(value.excludeCredentials) &&
    selection.userVerification === 'required' &&
    (selection.authenticatorAttachment === undefined ||
      selection.authenticatorAttachment === 'platform' ||
      selection.authenticatorAttachment === 'cross-platform') &&
    (selection.residentKey === undefined ||
      selection.residentKey === 'required' ||
      selection.residentKey === 'preferred' ||
      selection.residentKey === 'discouraged') &&
    (selection.requireResidentKey === undefined ||
      typeof selection.requireResidentKey === 'boolean') &&
    (value.timeout === undefined || (typeof value.timeout === 'number' && value.timeout > 0)) &&
    (value.attestation === undefined || typeof value.attestation === 'string')
  );
}

function isRequest(
  value: Record<string, unknown>,
): value is Record<string, unknown> & WebauthnRequestOptionsJSON {
  return (
    typeof value.challenge === 'string' &&
    value.rpId === location.hostname &&
    value.userVerification === 'required' &&
    descriptors(value.allowCredentials) &&
    (value.timeout === undefined || (typeof value.timeout === 'number' && value.timeout > 0))
  );
}

export function hostedCreationOptions(value: unknown): PublicKeyCredentialCreationOptions {
  const options = record(record(value).publicKey);
  if (!isCreation(options)) throw new Error('Invalid Identity WebAuthn registration options.');
  return parseCreationOptions(options);
}

export function hostedRequestOptions(value: unknown): PublicKeyCredentialRequestOptions {
  const options = record(record(value).publicKey);
  if (!isRequest(options)) throw new Error('Invalid Identity WebAuthn authentication options.');
  return parseRequestOptions(options);
}
