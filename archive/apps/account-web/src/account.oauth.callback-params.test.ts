import { describe, expect, it } from 'vite-plus/test';
import {
  assertGrantedAccountScopes,
  parseAccountOAuthCallback,
} from './account.oauth.callback-params';

describe('Account OAuth callback validation', () => {
  it('reads a complete authorization response', () => {
    expect(parseAccountOAuthCallback('?code=code-1&state=state-1')).toEqual({
      code: 'code-1',
      state: 'state-1',
    });
  });

  it('rejects provider errors and incomplete responses', () => {
    expect(() =>
      parseAccountOAuthCallback('?error=access_denied&error_description=Acc%C3%A8s+refus%C3%A9'),
    ).toThrow('Accès refusé');
    expect(() => parseAccountOAuthCallback('?code=code-1')).toThrow('incomplète');
  });

  it('requires every Account resource scope', () => {
    expect(() =>
      assertGrantedAccountScopes('account:profile:read account:privacy:read', [
        'account:profile:read',
        'account:privacy:read',
      ]),
    ).not.toThrow();

    expect(() =>
      assertGrantedAccountScopes('account:profile:read', [
        'account:profile:read',
        'account:profile:write',
      ]),
    ).toThrow('account:profile:write');
  });
});
