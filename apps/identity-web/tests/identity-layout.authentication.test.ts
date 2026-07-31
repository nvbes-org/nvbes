import { HttpError } from '@nvbes/http-client';
import { describe, expect, it } from 'vite-plus/test';

import { identityAuthenticationDisposition } from '../src/components/identity-layout.authentication';

describe('account layout authentication disposition', () => {
  it('redirects an unauthenticated account route to login', () => {
    expect(identityAuthenticationDisposition({ status: 401 })).toBe('redirect-login');
  });

  it('preserves the explicit reauthentication recovery screen', () => {
    const response = new Response('{}', { status: 401, statusText: 'Unauthorized' });
    const error = new HttpError('Session expired', response, {
      error: { recovery: 'reauthenticate' },
    });

    expect(identityAuthenticationDisposition(error)).toBe('reauthenticate');
  });

  it('does not redirect unrelated failures', () => {
    expect(identityAuthenticationDisposition({ status: 503 })).toBe('none');
  });
});
