import { describe, expect, it } from 'vite-plus/test';
import { renderToStaticMarkup } from 'react-dom/server';
import { LoginPageChooser } from '../src/pages/LoginPageChooser';
import type { AccountEntry } from '@nvbes/identity-client';

function account(status: 'active' | 'expired'): AccountEntry {
  return {
    authuser: '0',
    status,
    message: status === 'expired' ? 'Session expirée, veuillez vous reconnecter.' : null,
    user: {
      id: 'user-1',
      email: 'expired@example.test',
      display_name: 'Expired User',
      firstname: null,
      lastname: null,
      username: null,
      birthdate: null,
      region: null,
      email_verified: true,
      mfa_enabled: false,
      created_at: '2026-06-09T10:00:00Z',
    },
    session: {
      id: 'session-1',
      tenant_id: null,
      organization_id: null,
      workspace_id: null,
      workspace_region: null,
      created_at: '2026-06-09T10:00:00Z',
      last_seen_at: '2026-06-09T10:00:00Z',
      expires_at: '2026-06-09T11:00:00Z',
      revoked_at: null,
      ip: null,
      user_agent: null,
      device_id: null,
      device_trust_level: 'untrusted',
      device_trust_score: 0,
      risk_score: 0,
      risk_decision: 'allow',
      current: false,
    },
  };
}

describe('LoginPageChooser', () => {
  it('keeps expired accounts visible with a reconnect message', () => {
    const markup = renderToStaticMarkup(
      <LoginPageChooser
        accounts={[account('expired')]}
        onAccountSelect={() => undefined}
        onDisconnectAccount={() => undefined}
        onDisconnectAllAccounts={() => undefined}
        onUseAnotherAccount={() => undefined}
      />,
    );

    expect(markup).toContain('expired@example.test');
    expect(markup).toContain('Session expirée, veuillez vous reconnecter.');
    expect(markup).toContain('Se reconnecter');
  });
});
