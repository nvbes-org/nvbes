import { fetchPowChallenge, solvePowChallenge } from '@nvbes/identity-sdk-web';

export type PowChallengeProof = {
  powNonce: string;
  powSolution: string;
};

export async function resolvePowChallenge(identityApiBaseUrl: string): Promise<PowChallengeProof> {
  const challenge = await fetchPowChallenge(identityApiBaseUrl);
  if (!challenge.nonce || challenge.difficulty <= 0) {
    throw new Error('PoW challenge is required.');
  }

  return {
    powNonce: challenge.nonce,
    powSolution: String(await solvePowChallenge(challenge.nonce, challenge.difficulty)),
  };
}
