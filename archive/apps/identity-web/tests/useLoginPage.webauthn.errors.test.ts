import { describe, expect, it } from 'vite-plus/test';
import { applyConditionalWebAuthnResult } from '../src/pages/useLoginPage.webauthn.conditional';

describe('useLoginPage WebAuthn Error Handling', () => {
  it('sets an error when next_step is pwd but state_token or email is missing', async () => {
    let errorMessage: string | null = null;

    await applyConditionalWebAuthnResult(
      {
        next_step: 'pwd',
      },
      {
        finishLogin: async () => undefined,
        setSessionToken: () => undefined,
        setEmail: () => undefined,
        setLoginStateToken: () => undefined,
        setStep: () => undefined,
        setError: (msg) => {
          errorMessage = msg;
        },
      },
    );

    expect(errorMessage).toBe('La session de connexion est invalide. Veuillez réessayer.');
  });

  it('clears errors on successful password step transition', async () => {
    let errorMessage: string | null = 'Previous error';
    let currentStep = 'identifier';

    await applyConditionalWebAuthnResult(
      {
        next_step: 'pwd',
        state_token: 'valid-state-token',
        email: 'user@example.test',
      },
      {
        finishLogin: async () => undefined,
        setSessionToken: () => undefined,
        setEmail: () => undefined,
        setLoginStateToken: () => undefined,
        setStep: (step) => {
          currentStep = step;
        },
        setError: (msg) => {
          errorMessage = msg;
        },
      },
    );

    expect(currentStep).toBe('password');
    expect(errorMessage).toBeNull();
  });
});
