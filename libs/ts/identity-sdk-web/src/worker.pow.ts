export interface WorkerPowMessage {
  nonce: string;
  difficulty: number;
  batchSize?: number;
  yieldDelayMs?: number;
}

export interface WorkerPowProgress {
  type: 'progress' | 'complete';
  solution?: number;
  hashesComputed: number;
  hashesPerSecond: number;
  estimatedDutyCycle: number;
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

self.onmessage = async (event: MessageEvent<WorkerPowMessage>) => {
  const { nonce, difficulty } = event.data;
  let batchSize = event.data.batchSize ?? 500;
  let yieldDelayMs = event.data.yieldDelayMs ?? 15;

  const encoder = new TextEncoder();
  let solution = 0;
  let hashesComputed = 0;
  const startTime = performance.now();
  let lastProgressTime = startTime;
  let hashesInSample = 0;

  while (true) {
    const batchStart = performance.now();

    for (let i = 0; i < batchSize; i++) {
      const input = `${nonce}:${solution}`;
      const hashBuffer = await crypto.subtle.digest('SHA-256', encoder.encode(input));
      const hash = new Uint8Array(hashBuffer);

      if (leadingZeroBits(hash) >= difficulty) {
        self.postMessage({
          type: 'complete',
          solution,
          hashesComputed: hashesComputed + i + 1,
          hashesPerSecond: Math.round(
            ((hashesComputed + i + 1) / (performance.now() - startTime)) * 1000,
          ),
          estimatedDutyCycle: 0,
        } as WorkerPowProgress);
        return;
      }

      solution++;
    }

    const batchEnd = performance.now();
    const computeDuration = batchEnd - batchStart;
    hashesComputed += batchSize;
    hashesInSample += batchSize;

    // Report progress periodically (every ~300ms)
    if (batchEnd - lastProgressTime >= 300) {
      const sampleDuration = (batchEnd - lastProgressTime) / 1000;
      const hashesPerSecond = Math.round(hashesInSample / sampleDuration);
      const totalDuration = computeDuration + yieldDelayMs;
      const dutyCycle = totalDuration > 0 ? computeDuration / totalDuration : 0.2;

      self.postMessage({
        type: 'progress',
        hashesComputed,
        hashesPerSecond,
        estimatedDutyCycle: Math.round(dutyCycle * 100) / 100,
      } as WorkerPowProgress);

      lastProgressTime = batchEnd;
      hashesInSample = 0;
    }

    // Adaptive yield pause to prevent high CPU duty cycle
    if (yieldDelayMs > 0) {
      await new Promise((resolve) => setTimeout(resolve, yieldDelayMs));
    }
  }
};
