import { describe, expect, it } from 'vite-plus/test';

import {
  defaultNotificationPath,
  notificationUrl,
} from '../src/identity.service-worker.notification-url';

const origin = 'https://identity.nvbes.com';
const fallbackUrl = `${origin}${defaultNotificationPath}`;

describe('service worker notification URL', () => {
  it('accepts relative and same-origin absolute URLs', () => {
    expect(notificationUrl('/login', origin)).toBe(`${origin}/login`);
    expect(notificationUrl(`${origin}/security`, origin)).toBe(`${origin}/security`);
  });

  it.each([
    ['an external URL', 'https://attacker.example/phishing'],
    ['a protocol-relative external URL', '//attacker.example/phishing'],
    ['a javascript URL', 'javascript:alert(1)'],
    ['a blob URL', `blob:${origin}/notification`],
    ['a data URL', 'data:text/html,phishing'],
    ['an invalid URL', 'https://[invalid'],
    ['a missing URL', undefined],
  ])('uses the internal fallback for %s', (_label, value) => {
    expect(notificationUrl(value, origin)).toBe(fallbackUrl);
  });
});
