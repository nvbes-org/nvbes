export type MfaMethod = 'totp' | 'email' | 'webauthn' | 'recovery';

export function normalizeMfaMethods(methods: string[] | null | undefined): MfaMethod[] {
  return (methods ?? [])
    .map((method) => method as MfaMethod)
    .filter(
      (method): method is MfaMethod =>
        method === 'totp' || method === 'email' || method === 'webauthn' || method === 'recovery',
    );
}
