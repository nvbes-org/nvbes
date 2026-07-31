import { describe, expect, it } from 'vite-plus/test';

import { applyConditionalWebAuthnResult } from '../src/pages/useLoginPage.webauthn.conditional';

describe('applyConditionalWebAuthnResult', () => {
  it('continues with the password only after a valid WebAuthn result', async () => {
    let email = '';
    let stateToken: string | null = null;
    let step = 'identifier';
    let finishLoginCalled = false;

    await applyConditionalWebAuthnResult(
      {
        next_step: 'pwd',
        state_token: 'opaque-password-state',
        email: 'passkey@example.test',
      },
      {
        finishLogin: async () => {
          finishLoginCalled = true;
        },
        setSessionToken: () => undefined,
        setEmail: (value) => {
          email = value;
        },
        setLoginStateToken: (value) => {
          stateToken = value;
        },
        setStep: (value) => {
          step = value;
        },
        setError: () => undefined,
      },
    );

    expect(email).toBe('passkey@example.test');
    expect(stateToken).toBe('opaque-password-state');
    expect(step).toBe('password');
    expect(finishLoginCalled).toBe(false);
  });

  it('finishes the login when password skipping is enabled', async () => {
    let sessionToken: string | null = null;
    let finishedWith: string | null = null;

    await applyConditionalWebAuthnResult(
      { session_token: 'browser-session' },
      {
        finishLogin: async (session) => {
          finishedWith = session;
        },
        setSessionToken: (value) => {
          sessionToken = value;
        },
        setEmail: () => undefined,
        setLoginStateToken: () => undefined,
        setStep: () => undefined,
        setError: () => undefined,
      },
    );

    expect(sessionToken).toBe('browser-session');
    expect(finishedWith).toBe('browser-session');
  });
});
