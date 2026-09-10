import assert from 'node:assert/strict';
import { randomUUID } from 'node:crypto';

export async function verifyBillingAuthorization({
  fixture,
  origins,
  principalId,
  accountToken,
  billingToken,
  readOnlyToken,
}) {
  const request = async (path, expected, { token = billingToken, method = 'GET', body } = {}) => {
    const options = {
      method,
      signal: AbortSignal.timeout(5000),
      headers: { authorization: `Bearer ${token}`, 'content-type': 'application/json' },
    };
    if (body !== undefined) {
      assert.notEqual(method, 'GET');
      options.body = JSON.stringify(body);
    }
    const response = await fetch(`${origins.billing}${path}`, options);
    assert.equal(response.status, expected, `${method} ${path}`);
    await response.arrayBuffer();
  };
  const created = await fetch(`${origins.account}/api/v1/teams`, {
    method: 'POST',
    signal: AbortSignal.timeout(5000),
    headers: { authorization: `Bearer ${accountToken}`, 'content-type': 'application/json' },
    body: JSON.stringify({ name: 'Runtime billing authorization' }),
  });
  assert.equal(created.status, 201);
  const { id: team } = await created.json();
  assert.match(team, /^[0-9a-f-]{36}$/);
  assert.match(principalId, /^[0-9a-f-]{36}$/);
  const teamPath = `/accounts/team/${team}/billing`;
  const personalPath = `/accounts/principal/${principalId}/billing`;
  await request(`${teamPath}/overview`, 200);
  await request(`/workspaces/${team}/billing/overview`, 200);
  await request(`${personalPath}/overview`, 200);
  await request(`${teamPath}/checkout`, 403, {
    token: readOnlyToken,
    method: 'POST',
    body: { plan_code: 'standard_monthly' },
  });
  await request(`${teamPath}/checkout`, 400, {
    method: 'POST',
    body: { plan_code: 'standard_monthly', account_type: 'principal' },
  });
  for (const path of [
    `/accounts/principal/${randomUUID()}/billing`,
    `/accounts/team/${randomUUID()}/billing`,
    `/accounts/principal/${team}/billing`,
    `/workspaces/${principalId}/billing`,
  ]) {
    await request(`${path}/overview`, 403);
    await request(`${path}/portal`, 403, { method: 'POST' });
    await request(`${path}/checkout`, 403, {
      method: 'POST',
      body: { plan_code: 'standard_monthly' },
    });
  }
  // Simulate authoritative membership changes while keeping the same valid JWT.
  await fixture.sql(
    'account',
    `UPDATE account_team_memberships SET role='member' WHERE team_id='${team}' AND principal_id='${principalId}'`,
  );
  await request(`${teamPath}/overview`, 403);
  await request(`${teamPath}/portal`, 403, { method: 'POST' });
  await request(`${teamPath}/checkout`, 403, {
    method: 'POST',
    body: { plan_code: 'standard_monthly' },
  });
  await fixture.sql(
    'account',
    `UPDATE account_team_memberships SET role='owner' WHERE team_id='${team}' AND principal_id='${principalId}'`,
  );
  await request(`${teamPath}/overview`, 200);
  await fixture.sql('account', `UPDATE account_teams SET status='closed' WHERE id='${team}'`);
  await request(`${teamPath}/overview`, 403);
  await fixture.sql('account', `UPDATE account_teams SET status='active' WHERE id='${team}'`);
  await fixture.sql(
    'account',
    `UPDATE account_profiles SET lifecycle_status='closure_pending' WHERE principal_id='${principalId}'`,
  );
  await request(`${teamPath}/overview`, 403);
  await request(`${personalPath}/overview`, 403);
  await fixture.sql(
    'account',
    `UPDATE account_profiles SET lifecycle_status='active' WHERE principal_id='${principalId}'`,
  );
  await request(`${personalPath}/overview`, 200);
  assert.equal(await fixture.sql('billing', 'SELECT count(*) FROM billing_customers'), '0');
  assert.equal(await fixture.sql('billing', 'SELECT count(*) FROM billing_checkout_sessions'), '0');
  assert.equal(await fixture.sql('billing', 'SELECT count(*) FROM billing_audit_events'), '0');
  // Authorized mutations use Billing's local dummy provider; no Stripe request.
  await request(`${personalPath}/portal`, 200, { method: 'POST' });
  await request(`${teamPath}/portal`, 200, { method: 'POST' });
  await request(`${teamPath}/checkout`, 200, {
    method: 'POST',
    body: { plan_code: 'standard_monthly' },
  });
  assert.equal(
    await fixture.sql(
      'billing',
      "SELECT count(*) FROM billing_customers WHERE account_type='principal'",
    ),
    '1',
  );
  assert.equal(
    await fixture.sql(
      'billing',
      "SELECT count(*) FROM billing_customers WHERE account_type='team'",
    ),
    '1',
  );
  assert.equal(await fixture.sql('billing', 'SELECT count(*) FROM billing_checkout_sessions'), '1');
}
