import { HttpError } from '@nvbes/http-client';
import { describe, expect, it } from 'vite-plus/test';
import { ClientRuntimeError } from './index';
import { resolveAuthFallbackCopy } from './feature-fallbacks';

describe('auth fallback copy', () => {
  it('keeps authentication copy for unauthorized API responses', () => {
    const response = new Response(JSON.stringify({ error: { message: 'Unauthorized' } }), {
      status: 401,
      statusText: 'Unauthorized',
    });

    const copy = resolveAuthFallbackCopy(
      new ClientRuntimeError(
        'api',
        'Unauthorized',
        new HttpError('Unauthorized', response, { error: { message: 'Unauthorized' } }),
        401,
      ),
    );

    expect(copy).toEqual({
      title: "Probleme d'authentification",
      description:
        "Impossible de verifier votre identite. Reconnectez-vous ou verifiez votre session.",
    });
  });

  it('shows service unavailable copy for failed network fetches', () => {
    const copy = resolveAuthFallbackCopy(
      new ClientRuntimeError('unexpected', 'Failed to fetch', new TypeError('Failed to fetch')),
    );

    expect(copy).toEqual({
      title: 'Service indisponible',
      description:
        "Impossible de joindre le service requis. Verifiez que l'API est demarree puis reessayez.",
    });
  });
});
