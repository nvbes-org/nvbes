export interface PowChallenge {
  nonce: string;
  difficulty: number;
}

export async function solvePowChallenge(nonce: string, difficulty: number): Promise<number> {
  if (difficulty <= 0) return 0;

  try {
    const worker = new Worker(new URL('./worker.pow.ts', import.meta.url), { type: 'module' });
    const solution = await new Promise<number>((resolve, reject) => {
      worker.onmessage = (event: MessageEvent<{ solution: number }>) => {
        resolve(event.data.solution);
        worker.terminate();
      };
      worker.onerror = (err) => {
        reject(err);
        worker.terminate();
      };
      worker.postMessage({ nonce, difficulty });
    });
    return solution;
  } catch {
    return solveMainThread(nonce, difficulty);
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

async function solveMainThread(nonce: string, difficulty: number): Promise<number> {
  let solution = 0;
  while (true) {
    for (let i = 0; i < 5000; i++) {
      const input = `${nonce}:${solution}`;
      const hash = new Uint8Array(await crypto.subtle.digest('SHA-256', encoder.encode(input)));
      if (leadingZeroBits(hash) >= difficulty) return solution;
      solution++;
    }
    await new Promise((resolve) => setTimeout(resolve, 0));
  }
}

export async function fetchPowChallenge(baseUrl: string): Promise<PowChallenge | null> {
  try {
    const response = await fetch(`${baseUrl}/auth/challenge/pow`, {
      credentials: 'include',
    });
    if (!response.ok) return null;
    return response.json();
  } catch {
    return null;
  }
}
