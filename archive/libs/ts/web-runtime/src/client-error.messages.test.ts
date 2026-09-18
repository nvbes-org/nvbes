import { describe, expect, it } from 'vite-plus/test';
import { ClientRuntimeError, clientErrorMessage } from './index';

describe('client error presentation', () => {
  it.each([
    [400, 'Vérifiez les informations saisies.'],
    [401, 'Votre session a expiré. Reconnectez-vous.'],
    [403, 'Vous ne pouvez pas effectuer cette action.'],
    [404, 'La ressource demandée est introuvable.'],
    [409, 'Cette action entre en conflit avec une modification récente.'],
    [500, 'Une erreur est survenue. Veuillez réessayer.'],
    [599, 'Une erreur est survenue. Veuillez réessayer.'],
    [429, 'Fallback'],
    [undefined, 'Fallback'],
  ] as const)('maps HTTP %s without exposing backend details', (status, expected) => {
    expect(
      clientErrorMessage(
        new ClientRuntimeError('api', 'private backend detail', null, status),
        'Fallback',
      ),
    ).toBe(expected);
  });

  it.each([
    undefined,
    null,
    false,
    'raw',
    {},
    { error: null },
    { error: 'bad' },
    { error: {} },
    { error: { code: 42 } },
    { error: { code: 'unknown' } },
    { error: { code: 'toString' } },
    { error: { code: '__proto__' } },
    { error: { code: 'constructor' } },
  ])('rejects malformed and inherited error codes: %j', (body) => {
    expect(clientErrorMessage(new ClientRuntimeError('api', 'private', null, 403, body))).toBe(
      'Vous ne pouvez pas effectuer cette action.',
    );
  });

  it('prioritizes known codes, appends a request reference, and preserves non-API messages', () => {
    expect(
      clientErrorMessage(
        new ClientRuntimeError(
          'api',
          'private',
          null,
          500,
          { error: { code: 'step_up_required' } },
          'request-synthetic',
        ),
      ),
    ).toBe('Confirmez votre identité pour continuer. (ref: request-synthetic)');
    expect(clientErrorMessage(new Error('offline'))).toBe('offline');
    expect(clientErrorMessage(new Error(''), 'Try again')).toBe('Try again');
  });
});
