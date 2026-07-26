import { DeviceMonitor, DevicePerformanceState } from './device-monitor';

export interface PowChallenge {
  nonce: string;
  difficulty: number;
}

export interface PowSolverProgress {
  solution?: number;
  hashesComputed: number;
  hashesPerSecond: number;
  estimatedCpuDutyCycle: number;
  deviceState: DevicePerformanceState;
}

export interface PowSolverOptions {
  targetCpuLoad?: number;
  onProgress?: (progress: PowSolverProgress) => void;
}

export async function solvePowChallenge(
  nonce: string,
  difficulty: number,
  options?: PowSolverOptions,
): Promise<number> {
  const monitor = new DeviceMonitor(options?.targetCpuLoad ?? 0.2);
  const deviceState = monitor.getPerformanceState();

  try {
    const worker = new Worker(new URL('./worker.pow.ts', import.meta.url), {
      type: 'module',
    });

    return await new Promise<number>((resolve, reject) => {
      worker.onmessage = (
        event: MessageEvent<{
          type: string;
          solution?: number;
          hashesComputed: number;
          hashesPerSecond: number;
          estimatedDutyCycle: number;
        }>,
      ) => {
        const { type, solution, hashesComputed, hashesPerSecond, estimatedDutyCycle } = event.data;

        if (options?.onProgress) {
          options.onProgress({
            solution,
            hashesComputed,
            hashesPerSecond,
            estimatedCpuDutyCycle: estimatedDutyCycle ?? deviceState.targetCpuLoad,
            deviceState: monitor.getPerformanceState(),
          });
        }

        if (type === 'complete' && typeof solution === 'number') {
          resolve(solution);
          worker.terminate();
        }
      };

      worker.onerror = (err) => {
        reject(err);
        worker.terminate();
      };

      worker.postMessage({
        nonce,
        difficulty,
        batchSize: deviceState.recommendedBatchSize,
        yieldDelayMs: deviceState.recommendedYieldDelayMs,
      });
    });
  } catch {
    return solveMainThreadAdaptive(nonce, difficulty, monitor, options);
  }
}

function leadingZeroBits(hash: Uint8Array): number {
  let count = 0;
  for (let i = 0; i < hash.length; i++) {
    if (hash[i] === 0) {
      count += 8;
    } else {
      let b = hash[i];
      while ((b & 0x80) === 0) {
        count++;
        b <<= 1;
      }
      break;
    }
  }
  return count;
}

const encoder = new TextEncoder();

async function solveMainThreadAdaptive(
  nonce: string,
  difficulty: number,
  monitor: DeviceMonitor,
  options?: PowSolverOptions,
): Promise<number> {
  let solution = 0;
  let hashesComputed = 0;
  const startTime = performance.now();
  let lastProgressTime = startTime;
  let hashesInSample = 0;

  while (true) {
    const state = monitor.getPerformanceState();
    const batchSize = Math.min(state.recommendedBatchSize, 300); // Smaller batch for main thread 60fps safety
    const yieldDelayMs = Math.max(10, state.recommendedYieldDelayMs);

    for (let i = 0; i < batchSize; i++) {
      const input = `${nonce}:${solution}`;
      const hashBuffer = await crypto.subtle.digest('SHA-256', encoder.encode(input));
      const hash = new Uint8Array(hashBuffer);

      if (leadingZeroBits(hash) >= difficulty) {
        if (options?.onProgress) {
          options.onProgress({
            solution,
            hashesComputed: hashesComputed + i + 1,
            hashesPerSecond: Math.round(
              ((hashesComputed + i + 1) / (performance.now() - startTime)) * 1000,
            ),
            estimatedCpuDutyCycle: 0,
            deviceState: state,
          });
        }
        return solution;
      }
      solution++;
    }

    hashesComputed += batchSize;
    hashesInSample += batchSize;

    const now = performance.now();
    if (now - lastProgressTime >= 400) {
      if (options?.onProgress) {
        options.onProgress({
          hashesComputed,
          hashesPerSecond: Math.round((hashesInSample / (now - lastProgressTime)) * 1000),
          estimatedCpuDutyCycle: state.targetCpuLoad,
          deviceState: state,
        });
      }
      lastProgressTime = now;
      hashesInSample = 0;
    }

    // Yield to main UI thread using setTimeout to ensure 60fps responsiveness
    await new Promise((resolve) => setTimeout(resolve, yieldDelayMs));
  }
}

export async function fetchPowChallenge(baseUrl: string): Promise<PowChallenge> {
  const response = await fetch(`${baseUrl}/auth/challenge/pow`, {
    credentials: 'include',
  });
  if (!response.ok) {
    throw new Error(`PoW challenge request failed with status ${response.status}`);
  }

  return response.json();
}
