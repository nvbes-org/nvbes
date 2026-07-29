import { fetchPowChallenge, solvePowChallenge } from '@nvbes/identity-sdk-web';

import { createPowChallengePreloader } from './identity.auth.pow.preload';

export type PowChallengeProof = {
  powNonce: string;
  powSolution: string;
};

async function solvePowChallengeProof(accountServiceBaseUrl: string): Promise<PowChallengeProof> {
  const challenge = await fetchPowChallenge(accountServiceBaseUrl);
  if (!challenge.nonce || challenge.difficulty <= 0) {
    throw new Error('PoW challenge is required.');
  }

  return {
    powNonce: challenge.nonce,
    powSolution: String(await solvePowChallenge(challenge.nonce, challenge.difficulty)),
  };
}

const powChallengePreloader = createPowChallengePreloader(solvePowChallengeProof);

export function prefetchPowChallenge(accountServiceBaseUrl: string): void {
  powChallengePreloader.prefetch(accountServiceBaseUrl);
}

export function resolvePowChallenge(accountServiceBaseUrl: string): Promise<PowChallengeProof> {
  return powChallengePreloader.consume(accountServiceBaseUrl);
}
