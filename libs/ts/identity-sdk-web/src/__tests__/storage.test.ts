import { beforeEach, describe, expect, it } from 'vitest';
import { memoryStorage } from '../storage';

describe('MemoryStorage', () => {
  beforeEach(() => {
    memoryStorage.clearCodeVerifier();
    memoryStorage.clearState();
  });

  it('should save and retrieve code verifier', () => {
    memoryStorage.saveCodeVerifier('test-verifier');
    expect(memoryStorage.getCodeVerifier()).toBe('test-verifier');
  });

  it('should clear code verifier', () => {
    memoryStorage.saveCodeVerifier('test-verifier');
    memoryStorage.clearCodeVerifier();
    expect(memoryStorage.getCodeVerifier()).toBeNull();
  });

  it('should save and retrieve state', () => {
    memoryStorage.saveState('test-state');
    expect(memoryStorage.getState()).toBe('test-state');
  });

  it('should clear state', () => {
    memoryStorage.saveState('test-state');
    memoryStorage.clearState();
    expect(memoryStorage.getState()).toBeNull();
  });

  it('should start with null values after clear', () => {
    memoryStorage.clearCodeVerifier();
    memoryStorage.clearState();
    expect(memoryStorage.getCodeVerifier()).toBeNull();
    expect(memoryStorage.getState()).toBeNull();
  });
});
