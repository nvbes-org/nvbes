import { summarizeRollout } from './rollout-report.core.mjs';

export function validateRolloutPolicy(policy) {
  if (policy.version !== 1 || !['shadow', 'affected'].includes(policy.mode))
    throw new Error('Invalid rollout policy');
  if (
    !Number.isInteger(policy.minimumObservationDays) ||
    !Number.isInteger(policy.minimumRustPullRequests) ||
    policy.minimumObservationDays < 7 ||
    policy.minimumRustPullRequests < 20
  )
    throw new Error('Rollout observation thresholds cannot be reduced');
  if (policy.mode === 'affected') {
    const evidence = policy.activationEvidence;
    if (
      !evidence ||
      !Number.isInteger(evidence.comparedPrs) ||
      !Number.isFinite(evidence.observationDays) ||
      evidence.comparedPrs < policy.minimumRustPullRequests ||
      evidence.observationDays < policy.minimumObservationDays ||
      evidence.divergences !== 0 ||
      typeof evidence.reportCommit !== 'string' ||
      !/^[a-f0-9]{40}$/u.test(evidence.reportCommit)
    )
      throw new Error('Affected rollout needs reviewed, committed observation evidence');
  }
  return policy;
}

export function validateCommittedReport(policy, report) {
  if (policy.mode !== 'affected') return;
  if (report.rustEligibleForReview !== true || !Array.isArray(report.observations))
    throw new Error('Observation report is not eligible');
  const measured = summarizeRollout([{ evidence: report.observations }]);
  if (!measured.rustEligibleForReview)
    throw new Error('Committed observations do not meet the rollout thresholds');
  for (const field of ['comparedPrs', 'observationDays', 'divergences']) {
    if (report[field] !== policy.activationEvidence[field] || report[field] !== measured[field])
      throw new Error(`Committed rollout report disagrees on ${field}`);
  }
}
