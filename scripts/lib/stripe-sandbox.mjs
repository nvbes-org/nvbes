import { spawnSync } from 'node:child_process';

export const webhookEvents = [
  'checkout.session.completed',
  'customer.subscription.created',
  'customer.subscription.updated',
  'customer.subscription.deleted',
  'invoice.paid',
  'invoice.payment_failed',
];

export function validateSandboxConfig(env) {
  if (
    !/^(sk|rk)_test_[A-Za-z0-9]+$/.test(env.NVBES_STRIPE_SECRET_KEY ?? '') ||
    env.NVBES_STRIPE_SECRET_KEY.includes('dummy')
  ) {
    throw new Error('Set a dedicated sandbox API key in NVBES_STRIPE_SECRET_KEY (.env).');
  }
  if (!/^acct_[A-Za-z0-9]+$/.test(env.NVBES_STRIPE_ACCOUNT_ID ?? '')) {
    throw new Error('Set NVBES_STRIPE_ACCOUNT_ID to the expected sandbox account.');
  }
  if (env.NVBES_STRIPE_API_BASE_URL && env.NVBES_STRIPE_API_BASE_URL !== 'https://api.stripe.com') {
    throw new Error('Sandbox setup only sends credentials to https://api.stripe.com.');
  }
}

export async function stripeRequest(env, path, values, idempotencyKey) {
  validateSandboxConfig(env);
  const response = await fetch(`https://api.stripe.com/v1/${path}`, {
    method: values ? 'POST' : 'GET',
    headers: {
      Authorization: `Bearer ${env.NVBES_STRIPE_SECRET_KEY}`,
      'Stripe-Account': env.NVBES_STRIPE_ACCOUNT_ID,
      ...(idempotencyKey ? { 'Idempotency-Key': idempotencyKey } : {}),
    },
    body: values ? new URLSearchParams(values) : undefined,
    signal: AbortSignal.timeout(15000),
  });
  // Stripe response bodies can contain customer information; never log them on error.
  if (!response.ok)
    throw new Error(
      `Stripe ${path.split('?')[0]} failed (HTTP ${response.status}). Check key permissions in Workbench.`,
    );
  const result = await response.json();
  if (result.livemode === true) throw new Error('Live Stripe object rejected.');
  return result;
}

export async function verifySandbox(env) {
  const balance = await stripeRequest(env, 'balance');
  if (balance.livemode !== false) throw new Error('Stripe test mode could not be verified.');
  return env.NVBES_STRIPE_ACCOUNT_ID;
}

const ALLOWED_COMMANDS = new Set(['cargo', 'stripe', 'node']);

export function runPrivate(command, args, env, timeout = 30000) {
  if (!ALLOWED_COMMANDS.has(command)) {
    throw new Error(`Command ${command} is not allowed`);
  }
  if (!Array.isArray(args) || args.some((arg) => typeof arg !== 'string' || /[\r\n\0]/.test(arg))) {
    throw new Error('Invalid command arguments');
  }
  const result = spawnSync(command, args, { env, encoding: 'utf8', timeout });
  if (result.error) throw new Error(`${command} failed to start: ${result.error.code}.`);
  if (result.status !== 0) {
    const detail = (result.stderr || '').trim().split('\n').at(-1) || `exit ${result.status}`;
    throw new Error(
      `${command} failed: ${detail.replaceAll(env.NVBES_BILLING_DATABASE_URL || '', '<database-url>')}.`,
    );
  }
  return result.stdout;
}

export async function seedSandboxPlan(env) {
  const lookup = 'nvbes_sandbox_monthly_v1';
  const prices = await stripeRequest(env, `prices?lookup_keys[]=${lookup}&active=true`);
  let price = prices.data?.[0];
  if (!price) {
    const product = await stripeRequest(
      env,
      'products',
      {
        name: 'Nvbes — sandbox fixture (not a commercial offer)',
        'metadata[nvbes_fixture]': 'true',
      },
      'nvbes_sandbox_product_v1',
    );
    price = await stripeRequest(
      env,
      'prices',
      {
        product: product.id,
        currency: 'eur',
        unit_amount: '100',
        'recurring[interval]': 'month',
        lookup_key: lookup,
      },
      'nvbes_sandbox_price_v1',
    );
  }
  if (
    !/^price_[A-Za-z0-9]+$/.test(price.id) ||
    price.livemode !== false ||
    price.currency !== 'eur' ||
    price.unit_amount !== 100 ||
    price.recurring?.interval !== 'month'
  ) {
    throw new Error('Unexpected sandbox price; refusing to change Billing mappings.');
  }
  const sql = `INSERT INTO billing_plans (plan_code,name,stripe_price_id,currency,amount_cents,billing_interval)
    VALUES ('sandbox_monthly','Sandbox fixture','${price.id}','eur',100,'month')
    ON CONFLICT (plan_code) DO UPDATE SET stripe_price_id=EXCLUDED.stripe_price_id;
    UPDATE billing_plans SET is_active=false WHERE stripe_price_id LIKE 'price_test_%';`;
  const database = new URL(env.NVBES_BILLING_DATABASE_URL);
  runPrivate('psql', ['-X', '-v', 'ON_ERROR_STOP=1', '-c', sql], {
    ...env,
    PGHOST: database.hostname,
    PGPORT: database.port || '5432',
    PGDATABASE: database.pathname.slice(1),
    PGUSER: decodeURIComponent(database.username),
    PGPASSWORD: decodeURIComponent(database.password),
    PGCONNECT_TIMEOUT: '10',
  });
  return price.id;
}
