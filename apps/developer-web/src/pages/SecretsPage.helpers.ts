export type SecretRotationForm = {
  overlapHours: number;
};

export function buildSecretRotationPayload(form: SecretRotationForm) {
  if (!Number.isInteger(form.overlapHours) || form.overlapHours < 1) {
    throw new Error('Overlap must be at least 1 hour');
  }

  return { overlap_hours: form.overlapHours };
}
