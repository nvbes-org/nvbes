import { afterEach, expect, it, vi } from 'vite-plus/test';
import { DeviceMonitor } from './device-monitor';
afterEach(() => vi.unstubAllGlobals());
it.each([
  [0, undefined, 500],
  [2, 2, 500],
  [4, 2, 1000],
  [7, 8, 1000],
  [8, 3, 1000],
  [8, 4, 1500],
  [8, undefined, 1500],
])('sizes work for cores=%s memory=%s', (cores, memory, batch) => {
  vi.stubGlobal('navigator', { hardwareConcurrency: cores, deviceMemory: memory });
  expect(new DeviceMonitor().getPerformanceState()).toEqual({
    cpuCores: cores || 2,
    memoryGb: memory,
    battery: {},
    isLowPowerMode: false,
    targetCpuLoad: 0.2,
    recommendedBatchSize: batch,
    recommendedYieldDelayMs: 12,
  });
});
it.each([
  [0, 0.05, 57],
  [1, 0.4, 5],
  [0.1, 0.1, 27],
])('clamps CPU target %s', (target, load, delay) => {
  vi.stubGlobal('navigator', {});
  const result = new DeviceMonitor(target).getPerformanceState();
  expect(result.targetCpuLoad).toBe(load);
  expect(result.recommendedYieldDelayMs).toBe(delay);
});
it.each([2, 4, 8])(
  'updates battery state and limits discharging work (%s cores)',
  async (cores) => {
    const events = new Map<string, () => void>();
    const battery = {
      charging: false,
      level: 0.2,
      addEventListener: (name: string, handler: () => void) => events.set(name, handler),
    };
    vi.stubGlobal('navigator', { hardwareConcurrency: cores, getBattery: async () => battery });
    const monitor = new DeviceMonitor();
    await Promise.resolve();
    const low = monitor.getPerformanceState();
    expect(low.isLowPowerMode).toBe(true);
    expect(low.targetCpuLoad).toBe(0.08);
    expect(low.recommendedBatchSize).toBe(cores === 8 ? 750 : cores === 4 ? 500 : 250);
    expect(low.recommendedYieldDelayMs).toBe(35);
    battery.level = 0.21;
    events.get('levelchange')?.();
    expect(monitor.getPerformanceState().isLowPowerMode).toBe(false);
    battery.level = 0.1;
    events.get('levelchange')?.();
    battery.charging = true;
    events.get('chargingchange')?.();
    expect(monitor.getPerformanceState().isLowPowerMode).toBe(false);
    expect(low.battery).toEqual({ charging: false, level: 0.2 });
  },
);
it('tolerates rejected battery permission', async () => {
  vi.stubGlobal('navigator', { getBattery: () => Promise.reject(new Error('denied')) });
  const monitor = new DeviceMonitor();
  await Promise.resolve();
  await Promise.resolve();
  expect(monitor.getPerformanceState().battery).toEqual({});
});
