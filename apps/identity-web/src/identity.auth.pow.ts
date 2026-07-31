import { fetchPowChallenge, solvePowChallenge } from '@nvbes/identity-sdk-web';

import { createPowChallengePreloader } from './identity.auth.pow.preload';

export type PowChallengeProof = {
  powNonce: string;
  powSolution: string;
};

async function solvePowChallengeProof(identityServiceBaseUrl: string): Promise<PowChallengeProof> {
  const challenge = await fetchPowChallenge(identityServiceBaseUrl);
  if (!challenge.nonce || challenge.difficulty <= 0) {
    throw new Error('PoW challenge is required.');
  }

  return {
    powNonce: challenge.nonce,
    powSolution: String(await solvePowChallenge(challenge.nonce, challenge.difficulty)),
  };
}

const powChallengePreloader = createPowChallengePreloader(solvePowChallengeProof);

export function prefetchPowChallenge(identityServiceBaseUrl: string): void {
  powChallengePreloader.prefetch(identityServiceBaseUrl);
}

export function resolvePowChallenge(identityServiceBaseUrl: string): Promise<PowChallengeProof> {
  return powChallengePreloader.consume(identityServiceBaseUrl);
}
