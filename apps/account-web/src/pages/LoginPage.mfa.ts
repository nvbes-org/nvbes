export type MfaMethod = 'totp' | 'webauthn' | 'recovery';

export function normalizeMfaMethods(methods: string[] | null | undefined): MfaMethod[] {
  return (methods ?? [])
    .map((method) => method as MfaMethod)
    .filter(
      (method): method is MfaMethod =>
        method === 'totp' || method === 'webauthn' || method === 'recovery',
    );
}
