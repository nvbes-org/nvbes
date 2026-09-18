import { afterEach, describe, expect, it, vi } from 'vite-plus/test';
import {
  clearAnalyticsStorage,
  hasAnyAnalyticsConsent,
  normalizeAnalyticsError,
  sanitizeAnalyticsProperties,
} from './analytics.privacy';
import { EMPTY_ANALYTICS_CONSENT } from './analytics.types';

afterEach(() => vi.unstubAllGlobals());

describe('analytics privacy boundaries', () => {
  it.each(Object.keys(EMPTY_ANALYTICS_CONSENT))(
    'recognizes the independent %s purpose',
    (purpose) => {
      expect(hasAnyAnalyticsConsent({ ...EMPTY_ANALYTICS_CONSENT, [purpose]: true })).toBe(true);
    },
  );

  it.each([null, true, false, 0, -1, 1.5, 'safe', 'a'.repeat(200)])(
    'preserves safe primitive %j',
    async (value) => {
      expect(await sanitizeAnalyticsProperties({ variant: value }, vi.fn())).toEqual({
        variant: value,
      });
    },
  );

  it.each([
    NaN,
    Infinity,
    -Infinity,
    undefined,
    {},
    [],
    'a'.repeat(201),
    'user@example.test',
    'abc.def.ghi',
    '018f2f61-4875-7f7a-8bc8-8f70a73d2b1f',
  ])('drops unsafe property %j', async (value) => {
    expect(await sanitizeAnalyticsProperties({ variant: value }, vi.fn())).toEqual({});
  });

  it('pseudonymizes each supported identifier but never non-identifiers', async () => {
    const id = '018f2f61-4875-7f7a-8bc8-8f70a73d2b1f';
    const pseudonymize = vi.fn(async (prefix: string) => `${prefix}_opaque`);
    expect(
      await sanitizeAnalyticsProperties(
        { user_id: id, workspace_id: id, group_id: id },
        pseudonymize,
      ),
    ).toEqual({ user_id: 'usr_opaque', workspace_id: 'wks_opaque', group_id: 'wks_opaque' });
    expect(pseudonymize.mock.calls).toEqual([
      ['usr', id],
      ['wks', id],
      ['wks', id],
    ]);
    pseudonymize.mockClear();
    expect(
      await sanitizeAnalyticsProperties(
        { user_id: 42, workspace_id: 'invalid', group_id: null },
        pseudonymize,
      ),
    ).toEqual({});
    expect(pseudonymize).not.toHaveBeenCalled();
  });

  it('propagates pseudonymization failure without returning a raw identifier', async () => {
    const failure = new Error('crypto unavailable');
    await expect(
      sanitizeAnalyticsProperties({ user_id: '018f2f61-4875-7f7a-8bc8-8f70a73d2b1f' }, async () => {
        throw failure;
      }),
    ).rejects.toBe(failure);
  });

  it('bounds error messages and redacts emails and embedded tokens', () => {
    for (const message of ['failed user@example.test', 'Bearer abc.def.ghi was rejected']) {
      expect(normalizeAnalyticsError(new TypeError(message))).toEqual({
        name: 'TypeError',
        message: 'Redacted error message',
      });
    }
    expect(normalizeAnalyticsError(new Error('a'.repeat(161))).message).toBe('a'.repeat(160));
    expect(normalizeAnalyticsError({ secret: 'synthetic' })).toEqual({
      name: 'UnknownError',
      message: 'Unknown browser error',
    });
  });

  it.each(['account.nvbes.test', 'localhost', '127.0.0.1'])(
    'revokes only analytics storage and cookies on %s',
    (hostname) => {
      const localStorage = memoryStorage();
      const sessionStorage = memoryStorage();
      vi.stubGlobal('window', { location: { hostname }, localStorage, sessionStorage });
      const deletedCookies: string[] = [];
      vi.stubGlobal('document', {
        get cookie() {
          return '; PH_session=old; nvbes_analytics_id=old; csrf_token=keep; unrelated=keep';
        },
        set cookie(value: string) {
          deletedCookies.push(value);
        },
      });
      clearAnalyticsStorage();
      for (const storage of [localStorage, sessionStorage]) {
        expect(storage.getItem('ph_id')).toBeNull();
        expect(storage.getItem('NVBES.ANALYTICS.consent')).toBeNull();
        expect(storage.getItem('session')).toBe('keep');
        expect(storage.getItem('other.ph_id')).toBe('keep');
      }
      const domains = hostname === 'account.nvbes.test' ? [hostname, '.nvbes.test'] : [hostname];
      expect(deletedCookies).toEqual(
        ['PH_session', 'nvbes_analytics_id'].flatMap((name) => [
          `${name}=; Max-Age=0; path=/; SameSite=Lax`,
          ...domains.map((domain) => `${name}=; Max-Age=0; path=/; domain=${domain}; SameSite=Lax`),
        ]),
      );
    },
  );

  it('is harmless without a browser', () => {
    vi.stubGlobal('window', undefined);
    expect(() => clearAnalyticsStorage()).not.toThrow();
  });
});

function memoryStorage() {
  const values: Record<string, string> = {
    ph_id: 'old',
    'NVBES.ANALYTICS.consent': 'old',
    session: 'keep',
    'other.ph_id': 'keep',
  };
  return Object.assign(values, {
    getItem(key: string) {
      return values[key] ?? null;
    },
    removeItem(key: string) {
      delete values[key];
    },
  });
}
