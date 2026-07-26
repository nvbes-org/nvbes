import { describe, expect, it } from 'vite-plus/test';

import {
  findActiveAccount,
  getSwitcherMenuStyle,
  initialsForDisplayName,
  type SharedAccountOption,
} from './multi-account-switcher.utils';

describe('multi-account switcher utilities', () => {
  it('findActiveAccount prefers the active account and falls back to the first entry', () => {
    const accounts: SharedAccountOption[] = [
      { id: 'first', email: 'first@example.com', displayName: 'First User', isActive: false },
      { id: 'second', email: 'second@example.com', displayName: 'Second User', isActive: true },
    ];

    expect(findActiveAccount(accounts)?.id).toBe('second');
    expect(findActiveAccount([])).toBeNull();
    expect(findActiveAccount([{ ...accounts[0], isActive: false }])?.id).toBe('first');
  });

  it('initialsForDisplayName builds initials from up to two words and tolerates blanks', () => {
    expect(initialsForDisplayName('Ada Lovelace')).toBe('AL');
    expect(initialsForDisplayName('Single')).toBe('S');
    expect(initialsForDisplayName('  Grace   Brewster Murray  Hopper ')).toBe('GB');
    expect(initialsForDisplayName('')).toBe('');
  });

  it('getSwitcherMenuStyle keeps the menu wider than a narrow account trigger', () => {
    const style = getSwitcherMenuStyle({
      bottom: 128,
      left: 52,
      width: 260,
    });

    expect(style).toEqual({
      position: 'fixed',
      top: '132px',
      left: '52px',
      width: '384px',
    });
  });
});
