import { describe, expect, it, vi } from 'vitest';

import type { PowChallengeProof } from '../src/identity.auth.pow';
import { createPowChallengePreloader } from '../src/identity.auth.pow.preload';

function proof(powNonce: string): PowChallengeProof {
  return {
    powNonce,
    powSolution: '42',
  };
}

describe('createPowChallengePreloader', () => {
  it('reuses work started before the challenge is consumed', async () => {
    const resolve = vi.fn(async (baseUrl: string) => proof(baseUrl));
    const preloader = createPowChallengePreloader(resolve);

    preloader.prefetch('https://account.example');

    await expect(preloader.consume('https://account.example')).resolves.toEqual(
      proof('https://account.example'),
    );
    expect(resolve).toHaveBeenCalledOnce();
  });

  it('starts fresh work when the prefetched challenge is too old', async () => {
    let now = 0;
    const resolve = vi.fn(async (baseUrl: string) => proof(baseUrl));
    const preloader = createPowChallengePreloader(resolve, {
      maxAgeMs: 90_000,
      now: () => now,
    });

    preloader.prefetch('https://account.example');
    now = 90_000;

    await preloader.consume('https://account.example');
    expect(resolve).toHaveBeenCalledTimes(2);
  });

  it('retries after a speculative challenge fails', async () => {
    const resolve = vi
      .fn<(baseUrl: string) => Promise<PowChallengeProof>>()
      .mockRejectedValueOnce(new Error('temporary failure'))
      .mockResolvedValueOnce(proof('retry'));
    const preloader = createPowChallengePreloader(resolve);

    preloader.prefetch('https://account.example');
    await Promise.resolve();
    await Promise.resolve();

    await expect(preloader.consume('https://account.example')).resolves.toEqual(proof('retry'));
    expect(resolve).toHaveBeenCalledTimes(2);
  });
});
