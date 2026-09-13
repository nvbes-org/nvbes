import assert from 'node:assert/strict';
import { pathToFileURL } from 'node:url';

const ruleUid = 'nvbes-email-queue-stale';

export function idleQueueRule(rule, validationEnabled, datasourceUid) {
  assert.equal(validationEnabled, 'false', 'Reconciliation requires a closed validation window');
  assert.equal(rule.uid, ruleUid);
  assert.equal(rule.title, 'Email queue is stale');
  assert.equal(rule.condition, 'C');
  assert.equal(rule.for, '5m');
  assert.equal(rule.isPaused, false);
  assert.equal(rule.execErrState, 'Error');
  assert.ok(['Alerting', 'OK'].includes(rule.noDataState));
  assert.equal(rule.data.length, 3);
  const query = rule.data.find((data) => data.refId === 'A');
  assert.equal(query.datasourceUid, datasourceUid);
  assert.equal(query.model.expr, 'max(email_queue_oldest_age_seconds)');
  const reduce = rule.data.find((data) => data.refId === 'B');
  assert.equal(reduce.model.expression, 'A');
  assert.equal(reduce.model.reducer, 'last');
  const threshold = rule.data.find((data) => data.refId === 'C');
  assert.equal(threshold.model.expression, 'B');
  assert.equal(threshold.model.conditions[0].evaluator.type, 'gt');
  assert.deepEqual(threshold.model.conditions[0].evaluator.params, [300]);
  return { ...rule, noDataState: 'OK' };
}

async function reconcile() {
  assert.equal(process.env.GITHUB_REF, 'refs/heads/main');
  assert.equal(process.env.GITHUB_EVENT_NAME, 'workflow_dispatch');
  const url = new URL(process.env.GRAFANA_URL);
  assert.equal(url.protocol, 'https:');
  assert.equal(url.hostname, 'shayn.grafana.net');
  const token = process.env.GRAFANA_SERVICE_ACCOUNT_TOKEN;
  assert.ok(token, 'Grafana provisioning token is required');
  const endpoint = new URL(`/api/v1/provisioning/alert-rules/${ruleUid}`, url);
  const headers = {
    Authorization: `Bearer ${token}`,
    'Content-Type': 'application/json',
  };
  async function request(method, body) {
    const response = await fetch(endpoint, {
      method,
      headers,
      ...(body === undefined ? {} : { body: JSON.stringify(body) }),
      redirect: 'error',
      signal: AbortSignal.timeout(30_000),
    });
    assert.ok(response.ok, `Grafana ${method} failed: HTTP ${response.status}`);
    return response.json();
  }
  const before = await request('GET');
  const desired = idleQueueRule(
    before,
    process.env.EMAIL_SYNTHETIC_SMOKE_ENABLED,
    process.env.GRAFANA_PROMETHEUS_DATASOURCE_UID,
  );
  if (before.noDataState !== desired.noDataState) await request('PUT', desired);
  const after = await request('GET');
  idleQueueRule(
    after,
    process.env.EMAIL_SYNTHETIC_SMOKE_ENABLED,
    process.env.GRAFANA_PROMETHEUS_DATASOURCE_UID,
  );
  assert.equal(after.noDataState, 'OK');
  for (const key of [
    'data',
    'labels',
    'annotations',
    'notification_settings',
    'folderUID',
    'ruleGroup',
  ]) {
    assert.deepEqual(after[key], before[key], `Reconciliation unexpectedly changed ${key}`);
  }
  console.log(`${ruleUid}: noDataState=OK; threshold, routing and error handling preserved`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  await reconcile();
}
