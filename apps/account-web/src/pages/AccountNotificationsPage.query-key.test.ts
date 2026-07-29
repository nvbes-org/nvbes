import { describe, expect, it } from 'vite-plus/test';
import { getAccountNotificationsQueryKey } from './AccountNotificationsPage';

describe('getAccountNotificationsQueryKey', () => {
  it('isolates notification preferences by authuser', () => {
    expect(getAccountNotificationsQueryKey('0')).toEqual([
      'identity',
      'account',
      '0',
      'notifications',
    ]);
    expect(getAccountNotificationsQueryKey('7')).toEqual([
      'identity',
      'account',
      '7',
      'notifications',
    ]);
    expect(getAccountNotificationsQueryKey('0')).not.toEqual(getAccountNotificationsQueryKey('7'));
  });
});
