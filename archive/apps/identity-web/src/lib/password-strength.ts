import { ZxcvbnFactory } from '@zxcvbn-ts/core';
import { adjacencyGraphs, dictionary as commonDictionary } from '@zxcvbn-ts/language-common';
import { dictionary as frDictionary, translations } from '@zxcvbn-ts/language-fr';

const passwordStrengthEstimator = new ZxcvbnFactory({
  dictionary: {
    ...commonDictionary,
    ...frDictionary,
  },
  graphs: adjacencyGraphs,
  translations,
});

export function estimatePasswordStrength(password: string) {
  return passwordStrengthEstimator.check(password);
}
