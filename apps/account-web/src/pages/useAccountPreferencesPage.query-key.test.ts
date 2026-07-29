import { describe, expect, it } from 'vite-plus/test';
import { getAccountPreferencesQueryKey } from './useAccountPreferencesPage';

describe('getAccountPreferencesQueryKey', () => {
  it('isolates preferences by authuser', () => {
    expect(getAccountPreferencesQueryKey('0')).toEqual(['identity', 'account', '0', 'preferences']);
    expect(getAccountPreferencesQueryKey('7')).toEqual(['identity', 'account', '7', 'preferences']);
    expect(getAccountPreferencesQueryKey('0')).not.toEqual(getAccountPreferencesQueryKey('7'));
  });
});
