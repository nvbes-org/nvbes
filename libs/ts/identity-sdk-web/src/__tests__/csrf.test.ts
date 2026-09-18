import { afterEach, describe, expect, it } from 'vite-plus/test';
import { readCurrentAuthuser, readScopedCsrfToken } from '../csrf';

function installBrowserContext({
  cookie,
  pathname = '/',
  search = '',
}: {
  cookie: string;
  pathname?: string;
  search?: string;
}) {
  Object.defineProperty(globalThis, 'window', {
    configurable: true,
    value: {
      location: { pathname, search },
    },
  });
  Object.defineProperty(globalThis, 'document', {
    configurable: true,
    value: { cookie },
  });
}

afterEach(() => {
  Reflect.deleteProperty(globalThis, 'window');
  Reflect.deleteProperty(globalThis, 'document');
});

describe('readScopedCsrfToken', () => {
  it('reads the default account csrf token', () => {
    installBrowserContext({
      cookie: 'csrf_token=base; csrf_token_1=one',
    });

    expect(readScopedCsrfToken()).toBe('base');
  });

  it('reads a query-scoped csrf token', () => {
    installBrowserContext({
      cookie: 'csrf_token=base; csrf_token_1=one',
      search: '?authuser=1',
    });

    expect(readScopedCsrfToken()).toBe('one');
  });

  it('reads a path-scoped csrf token', () => {
    installBrowserContext({
      cookie: 'csrf_token=base; __Host-csrf_token_2=two',
      pathname: '/u/2/account/security',
    });

    expect(readScopedCsrfToken()).toBe('two');
  });

  it('reads a csrf token scoped by the canonical account route', () => {
    installBrowserContext({
      cookie: 'csrf_token_1=one; csrf_token_2=two',
      pathname: '/account/2/security',
      search: '?authuser=1',
    });

    expect(readScopedCsrfToken()).toBe('two');
  });
});

describe('readCurrentAuthuser', () => {
  it('reads the canonical account route before the legacy query parameter', () => {
    installBrowserContext({
      cookie: '',
      pathname: '/account/12/preferences',
      search: '?authuser=3',
    });

    expect(readCurrentAuthuser()).toBe('12');
  });

  it('keeps supporting the legacy query parameter', () => {
    installBrowserContext({
      cookie: '',
      pathname: '/account/preferences',
      search: '?authuser=3',
    });

    expect(readCurrentAuthuser()).toBe('3');
  });
});
