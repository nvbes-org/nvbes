import { describe, expect, it } from 'vite-plus/test';

import {
  clearVerificationSnapshot,
  readVerificationSnapshot,
  saveVerificationSnapshot,
} from '../src/identity.email-verification.state';

class MemoryStorage implements Storage {
  private values = new Map<string, string>();

  get length(): number {
    return this.values.size;
  }

  clear(): void {
    this.values.clear();
  }

  getItem(key: string): string | null {
    return this.values.get(key) ?? null;
  }

  key(index: number): string | null {
    return Array.from(this.values.keys())[index] ?? null;
  }

  removeItem(key: string): void {
    this.values.delete(key);
  }

  setItem(key: string, value: string): void {
    this.values.set(key, value);
  }
}

function withSessionStorage(run: () => void): void {
  const storage = new MemoryStorage();
  Object.defineProperty(globalThis, 'sessionStorage', {
    configurable: true,
    value: storage,
  });

  run();
}

describe('email verification snapshot', () => {
  it('keeps the current pending verification state in session storage', () => {
    withSessionStorage(() => {
      saveVerificationSnapshot({
        accountName: 'rayane',
        email: 'rayane@example.test',
        resendAvailableAt: '2026-07-01T10:00:30.000Z',
      });

      expect(readVerificationSnapshot()).toEqual({
        accountName: 'rayane',
        email: 'rayane@example.test',
        resendAvailableAt: '2026-07-01T10:00:30.000Z',
      });
    });
  });

  it('clears the pending verification state once the account is verified', () => {
    withSessionStorage(() => {
      saveVerificationSnapshot({
        accountName: 'rayane',
        email: 'rayane@example.test',
        resendAvailableAt: null,
      });

      clearVerificationSnapshot();

      expect(readVerificationSnapshot()).toBeNull();
    });
  });
});
