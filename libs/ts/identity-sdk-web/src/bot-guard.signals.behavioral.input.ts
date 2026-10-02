export function trackMouseEntropy() {
  if (navigator.maxTouchPoints > 0) {
    return {
      getCount: () => 0,
      getVariance: () => 999,
      getPathLength: () => 0,
      destroy: () => undefined,
    };
  }

  let count = 0;
  let pathLength = 0;
  const accelerations: number[] = [];
  let lastX = 0;
  let lastY = 0;
  let lastVelocityX = 0;
  let lastVelocityY = 0;
  let lastTimestamp = 0;

  const onMove = (event: MouseEvent) => {
    const elapsed = event.timeStamp - lastTimestamp;
    if (elapsed > 0 && lastTimestamp > 0) {
      const velocityX = (event.clientX - lastX) / elapsed;
      const velocityY = (event.clientY - lastY) / elapsed;
      const accelerationX = Math.abs(velocityX - lastVelocityX);
      const accelerationY = Math.abs(velocityY - lastVelocityY);
      accelerations.push(Math.sqrt(accelerationX ** 2 + accelerationY ** 2));
      if (accelerations.length > 50) accelerations.shift();
      lastVelocityX = velocityX;
      lastVelocityY = velocityY;
      pathLength += Math.sqrt((event.clientX - lastX) ** 2 + (event.clientY - lastY) ** 2);
    }
    lastX = event.clientX;
    lastY = event.clientY;
    lastTimestamp = event.timeStamp;
    count++;
  };

  document.addEventListener('mousemove', onMove, { passive: true });

  return {
    getCount: () => count,
    getVariance: () => variance(accelerations, 3) ?? 0,
    getPathLength: () => pathLength,
    destroy: () => document.removeEventListener('mousemove', onMove),
  };
}

export function trackKeyboardBiometrics(input: HTMLInputElement) {
  const keyDownTimes = new Map<string, number>();
  const dwellTimes: number[] = [];
  const flightTimes: number[] = [];
  let lastKeyUpTimestamp = 0;

  const onKeyDown = (event: KeyboardEvent) => {
    if (event.key.length !== 1 && event.key !== 'Backspace') return;
    keyDownTimes.set(event.key + event.timeStamp, event.timeStamp);
    if (lastKeyUpTimestamp > 0) {
      flightTimes.push(event.timeStamp - lastKeyUpTimestamp);
    }
  };

  const onKeyUp = (event: KeyboardEvent) => {
    for (const [key, timestamp] of keyDownTimes) {
      if (!key.startsWith(event.key)) continue;
      dwellTimes.push(event.timeStamp - timestamp);
      keyDownTimes.delete(key);
      break;
    }
    lastKeyUpTimestamp = event.timeStamp;
  };

  input.addEventListener('keydown', onKeyDown);
  input.addEventListener('keyup', onKeyUp);

  return {
    getDwellVariance: () => variance(dwellTimes, 3),
    getDwellMean: () => mean(dwellTimes),
    getFlightVariance: () => variance(flightTimes, 3),
    getFlightMean: () => mean(flightTimes),
    getSampleCount: () => Math.min(dwellTimes.length, flightTimes.length),
    destroy: () => {
      input.removeEventListener('keydown', onKeyDown);
      input.removeEventListener('keyup', onKeyUp);
    },
  };
}

function variance(values: number[], minimumSamples: number): number | null {
  if (values.length < minimumSamples) return null;
  const average = mean(values);
  if (average === null) return null;
  return values.reduce((sum, value) => sum + (value - average) ** 2, 0) / values.length;
}

function mean(values: number[]): number | null {
  if (values.length === 0) return null;
  return values.reduce((sum, value) => sum + value, 0) / values.length;
}
