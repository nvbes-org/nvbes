import { lanes } from './scope-plan.mjs';

export function gateFailures(needs) {
  const failures = [];
  if (needs.scope?.result !== 'success') failures.push('scope did not succeed');
  let plan;
  try {
    plan = JSON.parse(needs.scope?.outputs?.plan);
  } catch {
    failures.push('missing or invalid plan');
  }
  if (plan?.version !== 1) failures.push('unsupported plan version');
  if (needs['authorize-cache']?.result !== 'success')
    failures.push('cache authorization did not succeed');
  for (const lane of lanes) {
    const expected = plan?.candidate?.[lane];
    if (typeof expected !== 'boolean') failures.push(`${lane}: missing expectation`);
    const result = needs[lane]?.result;
    if (!result || !['success', 'skipped'].includes(result) || (expected && result !== 'success'))
      failures.push(
        `${lane}: expected ${expected ? 'success' : 'success or skipped'}, got ${result}`,
      );
  }
  return failures;
}
