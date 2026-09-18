import { METRICS, STATUSES, nonEmpty, unique, validateManifest } from './v1-manifest.mjs';

const canonicalDate = (value) =>
  typeof value === 'string' &&
  Number.isFinite(Date.parse(value)) &&
  new Date(value).toISOString() === value;

// Trust, file digests and producer provenance are verified by the I/O boundary.
// Missing verification must remain a failure even for a structurally valid packet.
export function evaluateV1({
  root,
  domains,
  bundle,
  units,
  expectedSha,
  manifestDigest,
  verified = false,
  now = new Date(),
  maxAgeHours = 24,
}) {
  const suites = validateManifest(root, domains);
  const failures = [];
  const rows = [];
  const fail = (message) => failures.push(message);
  if (!/^[a-f0-9]{40}$/u.test(expectedSha ?? '')) fail('Invalid candidate SHA');
  if (!verified) fail('Evidence signature, provenance and artifacts not verified');
  if (!bundle || bundle.schemaVersion !== 1) fail('Missing or unsupported evidence bundle');
  if (bundle?.sha !== expectedSha) fail('Evidence belongs to another SHA');
  if (!/^[a-f0-9]{64}$/u.test(manifestDigest ?? '') || bundle?.manifestDigest !== manifestDigest)
    fail('Evidence belongs to another manifest');
  if (!Number.isFinite(maxAgeHours) || maxAgeHours <= 0 || maxAgeHours > 24)
    fail('Invalid freshness policy');
  if (!(now instanceof Date) || !Number.isFinite(now.getTime())) fail('Invalid verification time');
  const results = Array.isArray(bundle?.results) ? bundle.results : [];
  if (!unique(results.map((result) => result.suite))) fail('Duplicate suite results');
  for (const result of results) {
    if (!suites.some((suite) => suite.id === result.suite)) fail(`Unknown suite: ${result.suite}`);
  }
  for (const suite of suites) {
    const result = results.find((entry) => entry.suite === suite.id);
    const status = result?.status ?? 'not-run';
    rows.push({ domain: suite.domain, suite: suite.id, status });
    if (!STATUSES.includes(status) || status !== 'passed') fail(`${suite.id}: ${status}`);
    if (!result) continue;
    if (result.producer?.kind !== (suite.execution === 'human' ? 'operator' : 'github-actions')) {
      fail(`${suite.id}: execution producer does not match declared suite`);
    }
    if (result.domain !== suite.domain || result.sha !== expectedSha)
      fail(`${suite.id}: wrong domain or SHA`);
    if (!['isolated', 'staging'].includes(result.environment))
      fail(`${suite.id}: invalid environment`);
    if (
      !canonicalDate(result.startedAt) ||
      !canonicalDate(result.completedAt) ||
      Date.parse(result.startedAt) > Date.parse(result.completedAt) ||
      Date.parse(result.completedAt) > now.getTime() ||
      now.getTime() - Date.parse(result.startedAt) > maxAgeHours * 3600000
    )
      fail(`${suite.id}: stale or invalid timestamps`);
    if (
      !result.tools ||
      Object.keys(result.tools).length === 0 ||
      !Object.values(result.tools).every(nonEmpty)
    )
      fail(`${suite.id}: missing tool versions`);
    if (!Array.isArray(result.artifacts) || result.artifacts.length === 0)
      fail(`${suite.id}: missing artifacts`);
    if (result.attempt !== 1) fail(`${suite.id}: retry cannot turn the gate green`);
    const cases = result.cases ?? [];
    if (
      !Array.isArray(cases) ||
      !unique(cases.map((entry) => entry.id)) ||
      cases.length !== suite.cases.length ||
      suite.cases.some((id) => !cases.some((entry) => entry.id === id && entry.status === 'passed'))
    )
      fail(`${suite.id}: incomplete test cases`);
  }
  if (!Array.isArray(units) || units.length === 0 || !unique(units.map((unit) => unit.name))) {
    fail('Missing or duplicate production measurement units');
  }
  const measurements = bundle?.measurements ?? [];
  if (!Array.isArray(measurements) || !unique(measurements.map((entry) => entry.unit))) {
    fail('Invalid or duplicate measurements');
  } else {
    for (const unit of units ?? []) {
      const measurement = measurements.find((entry) => entry.unit === unit.name);
      if (measurement?.sha !== expectedSha) fail(`${unit.name}: missing measurement for candidate`);
      if (
        !canonicalDate(measurement?.completedAt) ||
        Date.parse(measurement.completedAt) > now.getTime() ||
        now.getTime() - Date.parse(measurement.completedAt) > maxAgeHours * 3600000 ||
        !Array.isArray(measurement?.artifacts) ||
        measurement.artifacts.length === 0 ||
        !measurement.artifacts.every((file) =>
          bundle?.artifacts?.some((entry) => entry.path === file),
        )
      ) {
        fail(`${unit.name}: missing or stale measurement artifacts`);
      }
      for (const metric of METRICS) {
        const value = measurement?.[metric];
        const threshold = Math.max(root.thresholds[metric], unit.thresholds?.[metric] ?? 0);
        if (!Number.isFinite(value) || value < threshold || value > 100) {
          fail(`${unit.name}: ${metric} ${value ?? 'not-run'} (minimum ${threshold}%)`);
        }
      }
    }
  }
  for (const domain of domains) {
    for (const gap of domain.gaps)
      if (gap.status === 'open') fail(`${domain.domain}: open gap ${gap.id}`);
  }
  const ops = bundle?.operations;
  if (
    !ops ||
    !Number.isFinite(ops.rpoHours) ||
    ops.rpoHours < 0 ||
    ops.rpoHours > 24 ||
    !Number.isFinite(ops.rtoHours) ||
    ops.rtoHours < 0 ||
    ops.rtoHours > 8 ||
    !Number.isFinite(ops.targetMonthlyEurTtc) ||
    ops.targetMonthlyEurTtc < 0 ||
    ops.targetMonthlyEurTtc > 20 ||
    !Number.isFinite(ops.maximumMonthlyEurTtc) ||
    ops.maximumMonthlyEurTtc < ops.targetMonthlyEurTtc ||
    ops.maximumMonthlyEurTtc > 30
  )
    fail('Missing or failing restoration/FinOps evidence');
  return { verdict: failures.length ? 'NO-GO' : 'GO', failures, rows };
}
