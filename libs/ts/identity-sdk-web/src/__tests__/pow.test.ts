/// <reference types="node" />

import { beforeAll, describe, expect, it } from 'vite-plus/test';
import { DeviceMonitor } from '../device-monitor';
import { solvePowChallenge } from '../pow';

beforeAll(async () => {
  if (!globalThis.crypto) {
    const { webcrypto } = await import('node:crypto');
    Object.defineProperty(globalThis, 'crypto', {
      value: webcrypto,
      writable: true,
      configurable: true,
    });
  }
});

describe('Adaptive PoW & Device Monitor', () => {
  it('DeviceMonitor should return valid performance state', () => {
    const monitor = new DeviceMonitor(0.2);
    const state = monitor.getPerformanceState();

    expect(state.cpuCores).toBeGreaterThanOrEqual(1);
    expect(state.targetCpuLoad).toBe(0.2);
    expect(state.recommendedBatchSize).toBeGreaterThan(0);
    expect(state.recommendedYieldDelayMs).toBeGreaterThan(0);
  });

  it('solvePowChallenge should solve low-difficulty PoW with progress callbacks', async () => {
    let progressCount = 0;
    const solution = await solvePowChallenge('test-nonce', 4, {
      targetCpuLoad: 0.2,
      onProgress: (status) => {
        progressCount++;
        expect(status.deviceState).toBeDefined();
        expect(status.hashesComputed).toBeGreaterThan(0);
      },
    });

    expect(typeof solution).toBe('number');
    expect(solution).toBeGreaterThanOrEqual(0);
    expect(progressCount).toBeGreaterThan(0);
  });
});
