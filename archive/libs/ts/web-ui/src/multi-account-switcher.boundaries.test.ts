import { afterEach, expect, it, vi } from 'vite-plus/test';
import { getSwitcherMenuStyle, initialsForDisplayName } from './multi-account-switcher.utils';

afterEach(() => vi.unstubAllGlobals());
it('keeps the entire menu within the right viewport gutter', () => {
  vi.stubGlobal('innerWidth', 800);
  expect(getSwitcherMenuStyle({ bottom: 20, left: 700, width: 400 })).toEqual({
    position: 'fixed',
    top: '24px',
    left: '392px',
    width: '400px',
  });
  expect(getSwitcherMenuStyle({ bottom: 20, left: -10, width: 400 }).left).toBe('8px');
});
it.each([
  ['  ada    lovelace  ', 'AL'],
  ['\tgrace\n hopper extra', 'GH'],
  ['', ''],
])('normalizes display name %j', (name, initials) =>
  expect(initialsForDisplayName(name)).toBe(initials),
);
