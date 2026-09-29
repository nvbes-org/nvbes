import { expect, it } from 'vite-plus/test';
import { hostedDocument, identityBackend } from '../dev.routing';

it('separates interactive document, JSON, silent authorization and mutations', () => {
  const request = {
    method: 'GET',
    url: '/oauth/authorize?client_id=account',
    headers: { accept: 'text/html' },
  };
  expect(hostedDocument(request)).toBe(true);
  expect(hostedDocument({ ...request, headers: { accept: 'application/json' } })).toBe(false);
  expect(hostedDocument({ ...request, method: 'POST' })).toBe(false);
  expect(hostedDocument({ ...request, url: '/oauth/authorize?prompt=none' })).toBe(false);
  expect(hostedDocument({ ...request, url: '/oauth/authorize?prompt=login&prompt=none' })).toBe(
    false,
  );
  expect(hostedDocument({ ...request, url: '/oauth/token' })).toBe(false);
});

it('rejects remote backends, credentials and non-origin targets', () => {
  expect(identityBackend(undefined)).toBeUndefined();
  expect(identityBackend('http://127.0.0.1:3060')).toBe('http://127.0.0.1:3060');
  for (const value of [
    'https://example.com',
    'http://127.0.0.1:3060/path',
    'http://user@127.0.0.1:3060',
    'http://127.0.0.1:3060?query',
  ]) {
    expect(() => identityBackend(value)).toThrow();
  }
});
