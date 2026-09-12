import { afterEach, expect, it, vi } from 'vite-plus/test';
import { readVerifiedFetchCsrfToken } from './verified-fetch.csrf';

afterEach(() => vi.unstubAllGlobals());

it.each([
  ['?authuser=2', '/u/3', 'two'],
  ['', '/u/3/settings', 'three'],
  ['?authuser=invalid', '/u/3', 'three'],
  ['?authuser=0', '/u/3', 'default'],
  ['?authuser=1000', '/u/1234', 'default'],
  ['', '/unknown', 'default'],
  ['?authuser=23', '/', 'default'],
])('chooses a bounded account cookie for %s %s', (search, pathname, expected) => {
  vi.stubGlobal('window', { location: { search, pathname } });
  vi.stubGlobal('document', { cookie: 'csrf_token_2=two; csrf_token_3=three; csrf_token=default' });
  expect(readVerifiedFetchCsrfToken({})).toBe(expected);
});
it('prefers host-prefixed account cookies before default cookies', () => {
  vi.stubGlobal('window', { location: { search: '?authuser=2', pathname: '/' } });
  vi.stubGlobal('document', { cookie: 'csrf_token=default; __Host-csrf_token_2=account%20token' });
  expect(readVerifiedFetchCsrfToken({})).toBe('account token');
});
it('escapes custom cookie names and ignores similarly named cookies', () => {
  vi.stubGlobal('document', { cookie: 'axb=wrong; a.b=correct%2Bvalue; suffix_a.b=wrong' });
  expect(readVerifiedFetchCsrfToken({ csrfCookieName: 'a.b' })).toBe('correct+value');
  expect(readVerifiedFetchCsrfToken({ csrfCookieName: 'absent' })).toBeUndefined();
});
it('handles no DOM, no location, no cookie and an empty cookie', () => {
  vi.stubGlobal('window', undefined);
  vi.stubGlobal('document', undefined);
  expect(readVerifiedFetchCsrfToken({})).toBeUndefined();
  vi.stubGlobal('window', {});
  vi.stubGlobal('document', { cookie: 'csrf_token=; __Host-csrf_token=host' });
  expect(readVerifiedFetchCsrfToken({})).toBe('host');
  vi.stubGlobal('document', { cookie: '' });
  expect(readVerifiedFetchCsrfToken({})).toBeUndefined();
});
