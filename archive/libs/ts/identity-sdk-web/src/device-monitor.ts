export interface BatteryStatus {
  charging?: boolean;
  level?: number;
}

export interface DevicePerformanceState {
  cpuCores: number;
  memoryGb?: number;
  battery?: BatteryStatus;
  isLowPowerMode: boolean;
  targetCpuLoad: number;
  recommendedBatchSize: number;
  recommendedYieldDelayMs: number;
}

type BatteryNavigator = Navigator & {
  getBattery?: () => Promise<{
    charging: boolean;
    level: number;
    addEventListener: (type: string, listener: () => void) => void;
  }>;
  deviceMemory?: number;
};

export class DeviceMonitor {
  private targetCpuLoad: number;
  private batteryState: BatteryStatus = {};

  constructor(targetCpuLoad = 0.2) {
    // Clamp target CPU load between 5% (0.05) and 40% (0.40)
    this.targetCpuLoad = Math.max(0.05, Math.min(0.4, targetCpuLoad));
    this.initBatteryMonitoring();
  }

  private initBatteryMonitoring(): void {
    const nav = navigator as BatteryNavigator;
    if (typeof nav.getBattery === 'function') {
      nav
        .getBattery()
        .then((battery) => {
          this.batteryState = {
            charging: battery.charging,
            level: battery.level,
          };
          battery.addEventListener('chargingchange', () => {
            this.batteryState.charging = battery.charging;
          });
          battery.addEventListener('levelchange', () => {
            this.batteryState.level = battery.level;
          });
        })
        .catch(() => {
          // Battery API not supported or blocked
        });
    }
  }

  public getPerformanceState(): DevicePerformanceState {
    const nav = navigator as BatteryNavigator;
    const cpuCores = nav.hardwareConcurrency || 2;
    const memoryGb = nav.deviceMemory;

    // Detect if device is on low power mode or low battery (< 20% and discharging)
    const isLowPowerMode =
      this.batteryState.charging === false &&
      typeof this.batteryState.level === 'number' &&
      this.batteryState.level <= 0.2;

    // Adjust target CPU load based on battery status
    const effectiveCpuLoad = isLowPowerMode ? 0.08 : this.targetCpuLoad;

    // Dynamic batch size based on cores & memory
    let recommendedBatchSize = 500;
    if (cpuCores >= 8 && (!memoryGb || memoryGb >= 4)) {
      recommendedBatchSize = 1500;
    } else if (cpuCores >= 4) {
      recommendedBatchSize = 1000;
    }

    if (isLowPowerMode) {
      recommendedBatchSize = Math.max(250, Math.floor(recommendedBatchSize / 2));
    }

    // Calculate yield delay (pause) required to maintain effective CPU load
    // Duty cycle formula: Load = computeTime / (computeTime + yieldDelay)
    // -> yieldDelay = computeTime * (1 - Load) / Load
    // Assuming a batch takes ~2-5ms:
    const estimatedBatchTimeMs = 3;
    const recommendedYieldDelayMs = Math.round(
      Math.max(5, (estimatedBatchTimeMs * (1 - effectiveCpuLoad)) / effectiveCpuLoad),
    );

    return {
      cpuCores,
      memoryGb,
      battery: { ...this.batteryState },
      isLowPowerMode,
      targetCpuLoad: effectiveCpuLoad,
      recommendedBatchSize,
      recommendedYieldDelayMs,
    };
  }
}
