import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { join } from 'node:path';
import { generateKeyPairSync, randomBytes, randomUUID, sign, createHmac } from 'node:crypto';
import { setTimeout as delay } from 'node:timers/promises';
import { localDatabase } from './stripe-dev.mjs';
import {
  runPrivate,
  seedSandboxPlan,
  stripeRequest,
  verifySandbox,
} from './lib/stripe-sandbox.mjs';

const env = { ...process.env };
localDatabase(env.NVBES_BILLING_DATABASE_URL);
await verifySandbox(env);
const binary = join(process.env.CARGO_TARGET_DIR || 'target', 'debug', 'nvbes-billing-service');
runPrivate(binary, ['migrate'], env);
await seedSandboxPlan(env);
const { privateKey, publicKey } = generateKeyPairSync('rsa', { modulusLength: 2048 });
env.NVBES_IDENTITY_PUBLIC_KEY_PEM = publicKey.export({ type: 'spki', format: 'pem' });
env.NVBES_STRIPE_WEBHOOK_SECRET = `whsec_${randomBytes(32).toString('hex')}`;
env.NVBES_BILLING_BIND_ADDR = '127.0.0.1:3380';
env.NVBES_APP_URL = 'http://127.0.0.1:3380';
const backend = spawn(binary, ['serve'], { env, stdio: ['ignore', 'pipe', 'pipe'] });
// Consume runtime output without publishing provider responses or secrets.
backend.stdout.resume();
backend.stderr.resume();
const endpoint = 'http://127.0.0.1:3380';
let session;
try {
  let ready = false;
  for (let attempt = 0; attempt < 50; attempt++) {
    if (backend.exitCode !== null) throw new Error('Billing exited during startup.');
    try {
      ready = (await fetch(`${endpoint}/billing/plans`)).ok;
    } catch {}
    if (ready) break;
    await delay(100);
  }
  assert.ok(ready, 'Billing must start');
  const account = randomUUID();
  const encode = (object) => Buffer.from(JSON.stringify(object)).toString('base64url');
  const unsigned = `${encode({ alg: 'RS256', typ: 'JWT' })}.${encode({ sub: account, scope: 'billing:write', exp: Math.floor(Date.now() / 1000) + 300 })}`;
  const jwt = `${unsigned}.${sign('RSA-SHA256', Buffer.from(unsigned), privateKey).toString('base64url')}`;
  const headers = {
    Authorization: `Bearer ${jwt}`,
    'Content-Type': 'application/json',
    'Idempotency-Key': randomUUID(),
  };
  const checkout = async () => {
    const response = await fetch(`${endpoint}/workspaces/${account}/billing/checkout`, {
      method: 'POST',
      headers,
      body: JSON.stringify({ plan_code: 'sandbox_monthly', account_type: 'principal' }),
    });
    assert.equal(response.status, 200, 'Billing Checkout must succeed');
    return response.json();
  };
  session = await checkout();
  assert.deepEqual(await checkout(), session, 'Checkout must be idempotent');
  const provider = await stripeRequest(env, `checkout/sessions/${session.session_id}`);
  assert.equal(provider.livemode, false);
  assert.equal(provider.mode, 'subscription');
  assert.equal(provider.client_reference_id, account);
  const event = {
    id: `evt_contract_${randomUUID()}`,
    type: 'checkout.session.completed',
    livemode: false,
    created: Math.floor(Date.now() / 1000),
    data: { object: { id: session.session_id } },
  };
  const webhook = async (payload) => {
    const body = JSON.stringify(payload);
    const timestamp = Math.floor(Date.now() / 1000);
    const signature = createHmac('sha256', env.NVBES_STRIPE_WEBHOOK_SECRET)
      .update(`${timestamp}.${body}`)
      .digest('hex');
    return fetch(`${endpoint}/webhooks/stripe`, {
      method: 'POST',
      body,
      headers: {
        'Stripe-Signature': `t=${timestamp},v1=${signature}`,
        'Content-Type': 'application/json',
      },
    });
  };
  assert.equal((await webhook(event)).status, 200);
  assert.equal((await (await webhook(event)).json()).idempotent, true);
  assert.equal(
    (await webhook({ ...event, id: `evt_live_${randomUUID()}`, livemode: true })).status,
    400,
  );
  console.log(
    'Sandbox Billing smoke passed: real Stripe Checkout, idempotency, signed webhook replay, live rejection. No payment collected.',
  );
} finally {
  try {
    if (session)
      await stripeRequest(
        env,
        `checkout/sessions/${session.session_id}/expire`,
        {},
        `expire_${session.session_id}`,
      );
  } finally {
    backend.kill('SIGTERM');
    const timeout = setTimeout(() => backend.kill('SIGKILL'), 5000);
    timeout.unref();
  }
}
