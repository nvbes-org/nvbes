import { fetchPowChallenge, solvePowChallenge } from '@nvbes/identity-sdk-web';

export async function resolvePowChallenge(identityApiBaseUrl: string) {
  const challenge = await fetchPowChallenge(identityApiBaseUrl);
  if (!challenge || challenge.difficulty <= 0) {
    return {};
  }

  return {
    powNonce: challenge.nonce,
    powSolution: String(await solvePowChallenge(challenge.nonce, challenge.difficulty)),
  };
}
