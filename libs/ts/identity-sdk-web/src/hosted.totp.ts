import { hostedRequest, text, type HostedTransport } from './hosted.transport';

export interface HostedTotpEnrollment {
  factorId: string;
  secretBase32: string;
  provisioningUri: string;
  expiresAt: string;
}

const uuid = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;

function factorId(value: unknown): string {
  const id = text(value, 36);
  if (!uuid.test(id)) throw new Error('Invalid TOTP factor identifier.');
  return id;
}

function expiry(value: unknown): string {
  const result = text(value, 64);
  if (!/^\d{4}-\d{2}-\d{2}T/.test(result) || !Number.isFinite(Date.parse(result)))
    throw new Error('Invalid TOTP expiry.');
  return result;
}

function code(value: string): string {
  if (!/^[0-9]{6}$/.test(text(value, 6))) throw new Error('Invalid TOTP code.');
  return value;
}

/** Sensitive enrollment material: display on Identity only; never persist or log. */
export async function startHostedTotpEnrollment(
  config: HostedTransport,
  sessionCsrf: string,
): Promise<HostedTotpEnrollment> {
  const result = await hostedRequest(
    config,
    '/oauth/session/totp/enrollment/start',
    {},
    sessionCsrf,
  );
  const secret = text(result.secret_base32, 32);
  const uri = text(result.provisioning_uri, 512);
  // Accept only the active server profile, including a matching provisioning secret.
  // Do not expose URL parser errors: those can include the secret-bearing input.
  const match =
    /^otpauth:\/\/totp\/nvbes:([0-9a-f-]{36})\?secret=([A-Z2-7]{32})&issuer=nvbes&algorithm=SHA1&digits=6&period=30$/.exec(
      uri,
    );
  if (!/^[A-Z2-7]{32}$/.test(secret) || !match || !uuid.test(match[1] ?? '') || match[2] !== secret)
    throw new Error('Invalid TOTP provisioning response.');
  return {
    factorId: factorId(result.factor_id),
    secretBase32: secret,
    provisioningUri: uri,
    expiresAt: expiry(result.expires_at),
  };
}

export async function confirmHostedTotpEnrollment(
  config: HostedTransport,
  sessionCsrf: string,
  id: string,
  verificationCode: string,
): Promise<string> {
  const result = await hostedRequest(
    config,
    '/oauth/session/totp/enrollment/confirm',
    {
      factor_id: factorId(id),
      code: code(verificationCode),
    },
    sessionCsrf,
  );
  if (result.enrolled !== true || result.step_up !== true)
    throw new Error('Identity did not confirm TOTP enrollment.');
  return expiry(result.expires_at);
}

export async function stepUpHostedTotp(
  config: HostedTransport,
  sessionCsrf: string,
  verificationCode: string,
): Promise<string> {
  const result = await hostedRequest(
    config,
    '/oauth/session/step-up/totp',
    {
      code: code(verificationCode),
    },
    sessionCsrf,
  );
  if (result.step_up !== true) throw new Error('Identity did not confirm TOTP step-up.');
  return expiry(result.expires_at);
}
