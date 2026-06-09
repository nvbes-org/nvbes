/// <reference types="node" />

import assert from 'node:assert/strict';
import test from 'node:test';

import {
  findActiveAccount,
  initialsForDisplayName,
  type SharedAccountOption,
} from './multi-account-switcher.utils';

void test('findActiveAccount prefers the active account and falls back to the first entry', () => {
  const accounts: SharedAccountOption[] = [
    { id: 'first', email: 'first@example.com', displayName: 'First User', isActive: false },
    { id: 'second', email: 'second@example.com', displayName: 'Second User', isActive: true },
  ];

  assert.equal(findActiveAccount(accounts)?.id, 'second');
  assert.equal(findActiveAccount([]), null);
  assert.equal(findActiveAccount([{ ...accounts[0], isActive: false }])?.id, 'first');
});

void test('initialsForDisplayName builds initials from up to two words and tolerates blanks', () => {
  assert.equal(initialsForDisplayName('Ada Lovelace'), 'AL');
  assert.equal(initialsForDisplayName('Single'), 'S');
  assert.equal(initialsForDisplayName('  Grace   Brewster Murray  Hopper '), 'GB');
  assert.equal(initialsForDisplayName(''), '');
});
