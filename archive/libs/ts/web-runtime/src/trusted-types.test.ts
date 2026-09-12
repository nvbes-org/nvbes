import { afterEach, describe, expect, it, vi } from 'vite-plus/test';

afterEach(() => {
  Reflect.deleteProperty(globalThis, '__nvbesDefaultTrustedTypesPolicy');
  Reflect.deleteProperty(globalThis, '__nvbesTrustedTypesPolicy');
  vi.resetModules();
  vi.unstubAllGlobals();
});

describe('installDefaultTrustedTypesPolicy', () => {
  it('allows safe fallback values and rejects markup or cross-origin scripts', async () => {
    let createDefaultHtml: ((value: string) => string) | undefined;
    let createDefaultScriptUrl: ((value: string) => string) | undefined;
    vi.stubGlobal('location', new URL('https://identity.nvbes.com/register'));
    vi.stubGlobal('trustedTypes', {
      createPolicy: (
        name: string,
        rules: {
          createHTML(value: string): string;
          createScriptURL?(value: string): string;
        },
      ) => {
        if (name === 'default') {
          createDefaultHtml = (value) => rules.createHTML(value);
          createDefaultScriptUrl = (value) => rules.createScriptURL?.(value) ?? value;
        }
        return {
          createHTML: (value: string) => rules.createHTML(value),
          createScriptURL: (value: string) => rules.createScriptURL?.(value) ?? value,
        };
      },
    });

    const { installDefaultTrustedTypesPolicy } = await import('./trusted-types');
    installDefaultTrustedTypesPolicy();

    expect(createDefaultHtml?.('[data-radix-select-viewport]{display:none}')).toBe(
      '[data-radix-select-viewport]{display:none}',
    );
    expect(() => createDefaultHtml?.('<img src=x onerror=alert(1)>')).toThrow('markup-free');
    expect(createDefaultScriptUrl?.('/assets/pow.worker.js')).toBe(
      'https://identity.nvbes.com/assets/pow.worker.js',
    );
    expect(() => createDefaultScriptUrl?.('https://attacker.example/pow.worker.js')).toThrow(
      'default script',
    );
  });
});

describe('createTrustedServiceWorkerScriptUrl', () => {
  it('allows only the same-origin service worker entrypoint', async () => {
    vi.stubGlobal('location', new URL('https://identity.nvbes.com/login'));
    const { createTrustedServiceWorkerScriptUrl } = await import('./trusted-types');

    expect(createTrustedServiceWorkerScriptUrl('/sw.js')).toBe('/sw.js');
    expect(() => createTrustedServiceWorkerScriptUrl('https://attacker.example/sw.js')).toThrow(
      'service worker',
    );
    expect(() => createTrustedServiceWorkerScriptUrl('/other.js')).toThrow('service worker');
  });
});

describe('createTrustedWorkerScriptUrl', () => {
  it('allows same-origin worker modules and rejects external URLs', async () => {
    vi.stubGlobal('location', new URL('https://identity.nvbes.com/account'));
    vi.stubGlobal('trustedTypes', {
      createPolicy: (
        _name: string,
        rules: {
          createHTML(value: string): string;
          createScriptURL?(value: string): string;
        },
      ) => ({
        createHTML: (value: string) => rules.createHTML(value),
        createScriptURL: (value: string) => {
          const trustedValue = rules.createScriptURL?.(value) ?? value;
          return { toString: () => trustedValue };
        },
      }),
    });
    const { createTrustedWorkerScriptUrl } = await import('./trusted-types');

    expect(
      createTrustedWorkerScriptUrl(
        'https://identity.nvbes.com/assets/account.avatar-upload.worker.js',
      ).toString(),
    ).toBe('https://identity.nvbes.com/assets/account.avatar-upload.worker.js');
    expect(() =>
      createTrustedWorkerScriptUrl('https://attacker.example/account.avatar-upload.worker.js'),
    ).toThrow('worker script');
  });
});
