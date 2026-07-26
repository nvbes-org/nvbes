import { HttpError } from '@nvbes/http-client';
import { afterEach, describe, expect, it } from 'vite-plus/test';
import { z } from 'zod';
import {
  commandSearchTokenValue,
  detectSessionStale,
  detectVersionMismatch,
  findInvisibleUnicodeCharacters,
  formatRelativeDateTime,
  getSafeLocalStorage,
  inspectVersionMismatch,
  parseCommandSearch,
  recordVersionMismatchPrompt,
  revealInvisibleUnicodeCharacters,
  safeHtmlToString,
  sanitizeHtml,
  verifiedFetch,
  verifiedFetchJson,
  VerifiedFetchError,
} from './index';

type MemoryStorage = Pick<Storage, 'getItem' | 'setItem' | 'removeItem'>;

function createMemoryStorage(): MemoryStorage {
  const values: Record<string, string> = {};
  return {
    getItem: (key) => values[key] ?? null,
    removeItem: (key) => {
      delete values[key];
    },
    setItem: (key, value) => {
      values[key] = value;
    },
  };
}

function installWindow(localStorage = createMemoryStorage()) {
  Object.defineProperty(globalThis, 'window', {
    configurable: true,
    value: {
      localStorage,
      sessionStorage: createMemoryStorage(),
    },
    writable: true,
  });
  return localStorage;
}

afterEach(() => {
  Reflect.deleteProperty(globalThis, 'window');
  Reflect.deleteProperty(globalThis, 'document');
});

describe('safe storage', () => {
  it('validates stored JSON and tolerates unavailable storage', () => {
    const localStorage = installWindow();
    localStorage.setItem('session', JSON.stringify({ token: 'abc' }));

    const schema = z.object({ token: z.string() });
    expect(getSafeLocalStorage().getJson('session', schema)).toEqual({ token: 'abc' });

    Reflect.deleteProperty(globalThis, 'window');
    expect(getSafeLocalStorage().getItem('session')).toBeNull();
  });
});

describe('verified fetch', () => {
  it('adds the verification header and validates JSON responses', async () => {
    const calls: RequestInit[] = [];
    const fetchImpl: typeof fetch = async (_input, init) => {
      calls.push(init ?? {});
      return new Response(JSON.stringify({ ok: true }), { status: 200 });
    };

    await expect(
      verifiedFetchJson('/health', z.object({ ok: z.boolean() }), {
        fetchImpl,
        sameOrigin: 'https://app.nvbes.test',
      }),
    ).resolves.toEqual({ ok: true });

    const headers = new Headers(calls[0]?.headers);
    expect(headers.get('Nvbes-Verified-Fetch')).toBe('1');
  });

  it('marks mutating verified requests as XMLHttpRequest AJAX calls', async () => {
    const calls: RequestInit[] = [];
    const fetchImpl: typeof fetch = async (_input, init) => {
      calls.push(init ?? {});
      return new Response(JSON.stringify({ ok: true }), { status: 200 });
    };

    await verifiedFetch('/mutations', {
      body: JSON.stringify({ ok: true }),
      fetchImpl,
      method: 'POST',
      sameOrigin: 'https://app.nvbes.test',
      skipCsrf: true,
    });

    const headers = new Headers(calls[0]?.headers);
    expect(headers.get('X-Requested-With')).toBe('XMLHttpRequest');
  });

  it('refuses cross-origin requests unless the origin is allowed', async () => {
    const fetchImpl: typeof fetch = async () => new Response('{}');

    await expect(
      verifiedFetch('https://evil.example/api', {
        fetchImpl,
        sameOrigin: 'https://app.nvbes.test',
      }),
    ).rejects.toBeInstanceOf(VerifiedFetchError);

    await expect(
      verifiedFetch('https://api.nvbes.test/health', {
        allowedOrigins: ['https://api.nvbes.test'],
        fetchImpl,
        sameOrigin: 'https://app.nvbes.test',
      }),
    ).resolves.toBeInstanceOf(Response);
  });
});

describe('safe html and unicode inspection', () => {
  it('strips executable HTML before branding', () => {
    const html = sanitizeHtml('<p onclick="steal()">Hello</p><script>alert(1)</script>');
    expect(safeHtmlToString(html)).toBe('<p>Hello</p>');
  });

  it('finds and reveals hidden Unicode controls', () => {
    const value = 'client\u202e_id';
    expect(findInvisibleUnicodeCharacters(value)).toMatchObject([
      { codePoint: 'U+202E', name: 'right-to-left override' },
    ]);
    expect(revealInvisibleUnicodeCharacters(value)).toBe('client[U+202E]_id');
  });
});

describe('relative time and command search', () => {
  it('formats relative time from a fixed clock', () => {
    expect(formatRelativeDateTime('2026-06-21T12:05:00Z', new Date('2026-06-21T12:00:00Z'))).toBe(
      'in 5 minutes',
    );
  });

  it('parses query-builder tokens without losing free text', () => {
    const parsed = parseCommandSearch('failed user:usr_123 client:"portal app"');
    expect(parsed.text).toBe('failed');
    expect(commandSearchTokenValue(parsed, 'client')).toBe('portal app');
  });
});

describe('version mismatch detector', () => {
  it('rate limits reload prompts in local storage', () => {
    installWindow();
    expect(detectVersionMismatch({ backendReleaseId: 'api-2', frontendBuildId: 'web-1' })).toBe(
      true,
    );

    expect(
      inspectVersionMismatch({
        backendReleaseId: 'api-2',
        frontendBuildId: 'web-1',
        now: () => 1_000,
      }).shouldPrompt,
    ).toBe(true);

    recordVersionMismatchPrompt({ now: () => 1_000 });

    expect(
      inspectVersionMismatch({
        backendReleaseId: 'api-2',
        cooldownMs: 10_000,
        frontendBuildId: 'web-1',
        now: () => 2_000,
      }).shouldPrompt,
    ).toBe(false);
  });
});

describe('session stale detection', () => {
  it('does not infer a stale session from an unmarked 401 response', () => {
    const response = new Response('{}', { status: 401, statusText: 'Unauthorized' });
    const result = detectSessionStale(new HttpError('Unauthorized', response, {}));

    expect(result).toEqual({ reason: null, stale: false, status: 401 });
  });

  it('recognizes an explicit backend reauthentication requirement', () => {
    const response = new Response('{}', { status: 401, statusText: 'Unauthorized' });
    const result = detectSessionStale(
      new HttpError('Session expired', response, {
        error: { recovery: 'reauthenticate' },
      }),
    );

    expect(result).toEqual({ reason: 'reauthenticate', stale: true, status: 401 });
  });
});
