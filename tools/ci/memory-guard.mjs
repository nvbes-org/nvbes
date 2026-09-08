import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import os from 'node:os';

/**
 * Reads memory statistics from /proc/meminfo (Linux) or os module (macOS/other).
 */
export function getMemoryInfo() {
  try {
    const meminfo = readFileSync('/proc/meminfo', 'utf8');
    const getKb = (key) => {
      const match = meminfo.match(new RegExp(`^${key}:\\s+(\\d+)`, 'm'));
      return match ? Number.parseInt(match[1], 10) : null;
    };
    const totalKb = getKb('MemTotal');
    const availableKb = getKb('MemAvailable') ?? getKb('MemFree');
    const swapTotalKb = getKb('SwapTotal');
    const swapFreeKb = getKb('SwapFree');

    if (totalKb !== null && availableKb !== null) {
      const totalMb = Math.round(totalKb / 1024);
      const availableMb = Math.round(availableKb / 1024);
      const swapTotalMb = swapTotalKb ? Math.round(swapTotalKb / 1024) : 0;
      const swapFreeMb = swapFreeKb ? Math.round(swapFreeKb / 1024) : 0;
      const percentAvailable = Math.round((availableMb / totalMb) * 100);
      return {
        totalMb,
        availableMb,
        freeMb: availableMb,
        swapTotalMb,
        swapFreeMb,
        percentAvailable,
      };
    }
  } catch {
    // /proc/meminfo not available (e.g. macOS)
  }

  const totalMb = Math.round(os.totalmem() / (1024 * 1024));
  const availableMb = Math.round(os.freemem() / (1024 * 1024));
  const percentAvailable = totalMb > 0 ? Math.round((availableMb / totalMb) * 100) : 100;
  return {
    totalMb,
    availableMb,
    freeMb: availableMb,
    swapTotalMb: 0,
    swapFreeMb: 0,
    percentAvailable,
  };
}

/**
 * Computes recommended concurrency based on available system memory.
 */
export function recommendedParallelism({
  maxParallel = 4,
  minParallel = 1,
  memoryPerWorkerMb = 1536,
  silent = false,
} = {}) {
  const mem = getMemoryInfo();
  const cpus = os.cpus().length || 2;
  const memoryCapacityWorkers = Math.floor(mem.availableMb / memoryPerWorkerMb);
  const clampedWorkers = Math.max(
    minParallel,
    Math.min(maxParallel, Math.min(cpus, memoryCapacityWorkers || minParallel)),
  );

  if (!silent) {
    console.log(
      `[memory-guard] Total RAM: ${mem.totalMb} MB | Available: ${mem.availableMb} MB (${mem.percentAvailable}% free) | CPUs: ${cpus}. Sizing concurrency to ${clampedWorkers} workers (max: ${maxParallel}).`,
    );
  }

  return clampedWorkers;
}

let watchdogTimer = null;
let minAvailableMbSeen = Number.POSITIVE_INFINITY;

/**
 * Starts a background watchdog interval monitoring memory pressure.
 */
export function startMemoryWatchdog({
  intervalMs = 2000,
  thresholdMb = 400,
  onLowMemory = null,
} = {}) {
  if (watchdogTimer) return;
  minAvailableMbSeen = Number.POSITIVE_INFINITY;

  watchdogTimer = setInterval(() => {
    const mem = getMemoryInfo();
    if (mem.availableMb < minAvailableMbSeen) {
      minAvailableMbSeen = mem.availableMb;
    }

    if (mem.availableMb < thresholdMb) {
      console.warn(
        `\n[memory-guard: LOW MEMORY WARNING] Available memory dropped to ${mem.availableMb} MB (threshold: ${thresholdMb} MB)!`,
      );

      if (typeof globalThis.gc === 'function') {
        try {
          globalThis.gc();
          console.warn('[memory-guard] Triggered global.gc()');
        } catch {}
      }

      if (process.platform === 'linux' || process.platform === 'darwin') {
        try {
          const top = spawnSync('ps', ['-eo', 'pid,rss,comm', '--sort=-rss'], {
            encoding: 'utf8',
            timeout: 1000,
          });
          if (top.stdout) {
            const lines = top.stdout.trim().split('\n').slice(0, 6);
            console.warn('[memory-guard] Top memory-consuming processes:\n' + lines.join('\n'));
          }
        } catch {}
      }

      if (typeof onLowMemory === 'function') {
        try {
          onLowMemory(mem);
        } catch {}
      }
    }
  }, intervalMs);

  if (watchdogTimer.unref) {
    watchdogTimer.unref();
  }
}

/**
 * Stops the background memory watchdog and returns summary stats.
 */
export function stopMemoryWatchdog() {
  if (watchdogTimer) {
    clearInterval(watchdogTimer);
    watchdogTimer = null;
  }
  return { minAvailableMbSeen };
}

/**
 * Verifies if an exited process was terminated by the OOM killer.
 */
export function checkOomExit(status, signal, taskName = 'task') {
  const isOom = status === 137 || signal === 'SIGKILL';
  if (isOom) {
    console.error(
      `\n[memory-guard: OOM KILLER DETECTED] Process "${taskName}" was terminated by signal ${signal ?? 'SIGKILL'} (exit code ${status ?? 137}).`,
    );
    console.error(
      '[memory-guard] The kernel Out-Of-Memory (OOM) killer terminated this process because system RAM was exhausted.',
    );
    console.error(
      '[memory-guard] Suggestion: Decrease concurrency (--parallel) or allocate more memory.\n',
    );
  }
  return isOom;
}
