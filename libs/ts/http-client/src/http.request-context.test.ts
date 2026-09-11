import { afterEach, expect, it, vi } from 'vite-plus/test';
import {
  createIdempotencyKey,
  createRequestHeaders,
  currentAuthuser,
  readCsrfToken,
  resolveRequestUrl,
  sameOrigin,
  stripCrossOriginRequestContext,
} from './http.request-context';

afterEach(() => vi.unstubAllGlobals());

it('resolves relative paths with URL file semantics and absolute paths without a base', () => {
  expect(resolveRequestUrl('users/me', 'https://a.test/api').href).toBe('https://a.test/users/me');
  expect(resolveRequestUrl('https://b.test/item', 'invalid-base').href).toBe('https://b.test/item');
  expect(resolveRequestUrl('custom123://b.test/item', 'invalid-base').href).toBe(
    'custom123://b.test/item',
  );
});

it('uses UUIDs when Web Crypto provides them, including default request keys', () => {
  expect(createIdempotencyKey()).toMatch(/^[a-f0-9]{8}-(?:[a-f0-9]{4}-){3}[a-f0-9]{12}$/u);
  expect(createRequestHeaders('POST').get('Idempotency-Key')).toMatch(
    /^[a-f0-9]{8}-(?:[a-f0-9]{4}-){3}[a-f0-9]{12}$/u,
  );
});

it.each(['post', 'put', 'patch'])('adds both mutation headers for %s', (method) => {
  const headers = createRequestHeaders(method, undefined, 'explicit-key');
  expect([...headers]).toEqual([
    ['idempotency-key', 'explicit-key'],
    ['x-requested-with', 'XMLHttpRequest'],
  ]);
});

it.each(['GET', 'HEAD', 'OPTIONS'])('does not add mutation context for %s', (method) => {
  expect([...createRequestHeaders(method)]).toEqual([]);
});

it('strips every credential-bearing context header but preserves ordinary metadata', () => {
  const names = [
    'Authorization',
    'Proxy-Authorization',
    'Cookie',
    'DPoP',
    'X-Auth-User',
    'X-CSRF-Token',
    'Idempotency-Key',
    'X-Requested-With',
  ];
  const headers = new Headers(names.map((name) => [name, 'synthetic']));
  headers.set('Accept', 'application/json');
  stripCrossOriginRequestContext(headers);
  expect([...headers]).toEqual([['accept', 'application/json']]);
});

it.each([
  ['/account/123', '', '123'],
  ['/account/%31', '', '1'],
  ['/account/1234', '', undefined],
  ['/x/account/2', '', undefined],
  ['/account/', '', undefined],
  ['/account/abc', '?authuser=9', '9'],
  ['/settings', '?authuser=123', '123'],
  ['/settings', '?authuser=1234', undefined],
  ['/settings', '?authuser=1x', undefined],
  ['/u/23', '', '23'],
  ['/u/%32', '', '2'],
  ['/u/1234', '', undefined],
  ['/x/u/2', '', undefined],
  ['/u/', '', undefined],
  ['/account/1', '?authuser=2', '1'],
  ['/u/1', '?authuser=2', '2'],
  ['/settings', '?authuser=0', '0'],
])('resolves exact account selectors for %s %s', (pathname, search, expected) => {
  vi.stubGlobal('window', { location: { pathname, search } });
  expect(currentAuthuser()).toBe(expected);
});

it('accepts server environments and windows without a location', () => {
  vi.stubGlobal('window', undefined);
  expect(currentAuthuser()).toBeUndefined();
  vi.stubGlobal('window', {});
  expect(currentAuthuser()).toBeUndefined();
});

it.each([
  [undefined, 'csrf_token=first; __Host-csrf_token=second', 'first'],
  ['0', '__Host-csrf_token=host', 'host'],
  ['1', 'csrf_token=wrong; csrf_token_1=one', 'one'],
  ['1', '__Host-csrf_token_1=host-one', 'host-one'],
  ['2', 'csrf_token_1=wrong', undefined],
  [undefined, 'junk; =bad; csrf_token=abc==; csrf_token_1=wrong', 'abc=='],
  [undefined, 'csrf_token=; __Host-csrf_token=backup', 'backup'],
  [undefined, 'csrf_token=old; csrf_token=new', 'new'],
  [undefined, 'csrf_token=valid; csrf_token', 'valid'],
])('reads only the selected CSRF cookie for %s', (authuser, cookie, expected) => {
  vi.stubGlobal('document', { cookie });
  expect(readCsrfToken(authuser)).toBe(expected);
});

it('has no CSRF token outside a browser', () => {
  vi.stubGlobal('document', undefined);
  expect(readCsrfToken()).toBeUndefined();
});

it.each([
  ['/users', 'https://a.test/api///', 'https://a.test/api/users'],
  ['users', 'https://a.test/api/', 'https://a.test/api/users'],
  ['/users', 'https://a.test/', 'https://a.test/users'],
  ['///users', 'https://a.test/api/', 'https://a.test/api/users'],
  ['HTTPS://b.test/a', 'https://a.test/api/', 'https://b.test/a'],
  ['web+demo://b.test/a', 'https://a.test/api/', 'web+demo://b.test/a'],
  ['//b.test/users', 'https://a.test/', 'https://b.test/users'],
  [
    'nested/https://b.test/item',
    'https://a.test/api/',
    'https://a.test/api/nested/https://b.test/item',
  ],
])('resolves %s against %s', (path, base, expected) => {
  expect(resolveRequestUrl(path, base).href).toBe(expected);
});

it('origin checks include scheme and port but not path', () => {
  expect(sameOrigin('https://a.test/a', 'https://a.test:443/b')).toBe(true);
  expect(sameOrigin('http://a.test', 'https://a.test')).toBe(false);
  expect(sameOrigin('https://a.test:444', 'https://a.test')).toBe(false);
});

it('falls back to 16 random bytes encoded as exactly 32 hex digits', () => {
  vi.stubGlobal('crypto', {
    getRandomValues: (bytes: Uint8Array) => {
      expect(bytes).toHaveLength(16);
      bytes.set([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 254, 255]);
      return bytes;
    },
  });
  expect(createIdempotencyKey()).toBe('000102030405060708090a0b0c0dfeff');
});
