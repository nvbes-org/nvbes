import {
  completeHostedConsent,
  getHostedAuthenticationStatus,
  loadHostedAuthorization,
  loginHostedPasskey,
  loginHostedPassword,
  stepUpHostedPasskey,
  stepUpHostedTotp,
  listHostedPasskeys,
  listHostedTotpFactors,
  registerHostedPasskey,
  startHostedTotpEnrollment,
  confirmHostedTotpEnrollment,
  type HostedInteraction,
} from '@nvbes/identity-sdk-web/oauth';

export function identityGateway(origin: string) {
  const config = { baseUrl: origin };
  return {
    load: (url: string) => loadHostedAuthorization(config, url),
    status: (interaction: HostedInteraction) => getHostedAuthenticationStatus(config, interaction),
    password: (interaction: HostedInteraction, email: string, password: string) =>
      loginHostedPassword(config, interaction, { email, password }),
    passkey: (interaction: HostedInteraction) => loginHostedPasskey(config, interaction),
    stepUpPasskey: (csrf: string) => stepUpHostedPasskey(config, csrf),
    stepUpTotp: (csrf: string, code: string) => stepUpHostedTotp(config, csrf, code),
    hasFactors: async (csrf: string) => {
      const [passkeys, totp] = await Promise.all([
        listHostedPasskeys(config, csrf),
        listHostedTotpFactors(config, csrf),
      ]);
      return passkeys.length + totp.length > 0;
    },
    registerPasskey: (csrf: string, label: string) => registerHostedPasskey(config, csrf, label),
    startTotp: (csrf: string) => startHostedTotpEnrollment(config, csrf),
    confirmTotp: (csrf: string, factorId: string, code: string) =>
      confirmHostedTotpEnrollment(config, csrf, factorId, code),
    consent: (interaction: HostedInteraction, decision: 'approve' | 'deny') =>
      completeHostedConsent(config, interaction, decision),
  };
}
export type IdentityGateway = ReturnType<typeof identityGateway>;
