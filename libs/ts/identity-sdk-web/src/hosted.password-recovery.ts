import { hostedRequest, text, type HostedTransport } from './hosted.transport';

export async function loadPasswordRecovery(config: HostedTransport): Promise<string> {
  const response = await hostedRequest(config, '/oauth/password-recovery/context');
  const csrf = text(response.csrf_token, 43);
  if (!/^[A-Za-z0-9_-]{43}$/u.test(csrf)) throw new Error('Invalid recovery context');
  return csrf;
}

export async function requestPasswordRecovery(
  config: HostedTransport,
  csrf: string,
  email: string,
): Promise<void> {
  const response = await hostedRequest(
    config,
    '/oauth/password-recovery/request',
    { email: text(email, 320) },
    csrf,
  );
  if (response.accepted !== true) throw new Error('Recovery request not acknowledged');
}

export async function resetRecoveredPassword(
  config: HostedTransport,
  csrf: string,
  token: string,
  password: string,
): Promise<void> {
  if (!/^[A-Za-z0-9_-]{43}$/u.test(token)) throw new Error('Invalid recovery link');
  const response = await hostedRequest(
    config,
    '/oauth/password-recovery/reset',
    { token, password: text(password, 1024) },
    csrf,
  );
  if (response.reset !== true || response.must_reauthenticate !== true)
    throw new Error('Reset not acknowledged');
}
