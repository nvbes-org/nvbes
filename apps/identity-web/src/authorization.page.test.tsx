import { afterEach, describe, expect, it, vi } from 'vite-plus/test';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { AuthorizationPage } from './authorization.page';
import { AuthorizationController } from './authorization.controller';
import type { IdentityGateway } from './authorization.gateway';

afterEach(cleanup);

describe('Identity login screen', () => {
  it('submits accessible fields, clears the password, and renders server-required WebAuthn', async () => {
    const interaction = {
      interaction: 'id',
      csrfToken: 'csrf',
      sessionCsrfToken: null,
      needsLogin: true,
      clientId: 'Account',
      scope: 'openid',
    };
    let loggedIn = false;
    const gateway: IdentityGateway = {
      load: vi.fn().mockResolvedValue(interaction),
      status: vi.fn<IdentityGateway['status']>(async () => ({
        minimumAuthentication: 'recent_webauthn',
        needsLogin: !loggedIn,
        needsStepUp: loggedIn,
        proofExpiresAt: null,
      })),
      password: vi.fn(async () => {
        loggedIn = true;
        return {
          ...interaction,
          csrfToken: 'new-csrf',
          sessionCsrfToken: 'session',
          needsLogin: false,
        };
      }),
      passkey: vi.fn(),
      hasFactors: vi.fn().mockResolvedValue(true),
      recoveryCodes: vi.fn(),
      registerPasskey: vi.fn(),
      startTotp: vi.fn(),
      confirmTotp: vi.fn(),
      stepUpPasskey: vi.fn(),
      stepUpTotp: vi.fn(),
      consent: vi.fn(),
    };
    const controller = new AuthorizationController(gateway, vi.fn());
    await controller.start('authorization');
    render(<AuthorizationPage controller={controller} />);
    fireEvent.change(screen.getByLabelText('Adresse email'), {
      target: { value: 'user@example.invalid' },
    });
    const password = screen.getByLabelText<HTMLInputElement>('Mot de passe');
    fireEvent.change(password, { target: { value: 'never-persist-me' } });
    fireEvent.click(screen.getByRole('button', { name: 'Continuer' }));
    expect(password.value).toBe('');
    await waitFor(() =>
      expect(screen.getByRole('heading', { level: 1 }).textContent).toBe(
        'Confirmez que c’est vous.',
      ),
    );
    expect(gateway.password).toHaveBeenCalledExactlyOnceWith(
      interaction,
      'user@example.invalid',
      'never-persist-me',
    );
    expect(screen.queryByLabelText('Code à 6 chiffres')).toBeNull();
    expect(screen.queryByRole('button', { name: 'Autoriser et continuer' })).toBeNull();
    expect(screen.getByRole('button', { name: 'Vérifier avec une passkey' })).toBeTruthy();
  });
});
