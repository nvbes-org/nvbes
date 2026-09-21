import { spawn } from 'node:child_process';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import {
  runPrivate,
  seedSandboxPlan,
  verifySandbox,
  webhookEvents,
} from './lib/stripe-sandbox.mjs';

export function localDatabase(url) {
  const target = new URL(url);
  if (
    !['postgres:', 'postgresql:'].includes(target.protocol) ||
    !['localhost', '127.0.0.1', '[::1]', 'postgres'].includes(target.hostname) ||
    !/^\/nvbes_(dev_billing|billing_test[A-Za-z0-9_]*)$/.test(target.pathname)
  ) {
    throw new Error('Local Stripe setup requires an isolated local dev/test Billing database.');
  }
}

async function main() {
  const env = { ...process.env };
  const children = new Set();
  let stopping = false;
  function stop(code) {
    if (stopping) return;
    stopping = true;
    process.exitCode = code;
    for (const child of children) child.kill('SIGTERM');
    const timer = setTimeout(() => {
      for (const child of children) child.kill('SIGKILL');
    }, 5000);
    timer.unref();
  }
  process.on('SIGINT', () => stop(130));
  process.on('SIGTERM', () => stop(143));
  function start(command, args, childEnv, privateOutput = false) {
    const child = spawn(command, args, {
      env: childEnv,
      stdio: privateOutput ? ['ignore', 'pipe', 'pipe'] : 'inherit',
    });
    children.add(child);
    child.on('error', () => stop(1));
    child.on('exit', (code) => {
      children.delete(child);
      stop(code ?? 1);
    });
    return child;
  }
  localDatabase(env.NVBES_BILLING_DATABASE_URL);
  const mode = env.NVBES_STRIPE_DEV_MODE || 'sandbox';
  if (process.argv.includes('--check')) {
    if (mode === 'sandbox') await verifySandbox(env);
    else if (mode !== 'mock') throw new Error('NVBES_STRIPE_DEV_MODE must be sandbox or mock.');
    console.log(`Billing Stripe preflight: ${mode}.`);
    return;
  }
  if (mode === 'mock') {
    env.NVBES_STRIPE_SECRET_KEY = 'sk_test_dummy_key_for_testing';
    env.NVBES_STRIPE_WEBHOOK_SECRET = 'whsec_test_dummy_secret';
    console.log('Billing: explicit offline mock mode.');
  } else if (mode === 'sandbox') {
    const account = await verifySandbox(env);
    // Run migrations before seeding, including when started independently.
    runPrivate(
      'cargo',
      ['run', '--package', 'nvbes-billing-service', '--', 'migrate'],
      env,
      300000,
    );
    const price = await seedSandboxPlan(env);
    const cliEnv = { ...env, STRIPE_API_KEY: env.NVBES_STRIPE_SECRET_KEY };
    const cliArgs = [
      'listen',
      '--project-name',
      env.NVBES_STRIPE_CLI_PROJECT || 'nvbes-dev',
      '--events',
      webhookEvents.join(','),
      '--forward-to',
      `http://${env.NVBES_BILLING_BIND_ADDR || '127.0.0.1:3080'}/webhooks/stripe`,
    ];
    const listener = start('stripe', cliArgs, cliEnv, true);
    try {
      env.NVBES_STRIPE_WEBHOOK_SECRET = await new Promise((resolveSecret, reject) => {
        let buffer = '';
        const timer = setTimeout(
          () => reject(new Error('Stripe listener startup timed out.')),
          30000,
        );
        const receive = (chunk) => {
          buffer = (buffer + chunk.toString()).slice(-8192);
          const match = buffer.match(/whsec_[A-Za-z0-9]+/);
          if (match) {
            clearTimeout(timer);
            resolveSecret(match[0]);
          }
        };
        listener.stdout.on('data', receive);
        listener.stderr.on('data', receive);
        listener.once('exit', () => {
          clearTimeout(timer);
          reject(new Error('Stripe listener stopped. Check webhook permissions.'));
        });
      });
    } catch (error) {
      stop(1);
      throw error;
    }
    // Listener output is consumed privately: neither secrets nor webhook payloads are logged.
    const safeAccount = String(account).replace(/[\r\n]/g, '');
    const safePrice = String(price).replace(/[\r\n]/g, '');
    console.log(`Billing: sandbox ${safeAccount}, fixture ${safePrice}, webhook ready.`);
  } else {
    throw new Error('NVBES_STRIPE_DEV_MODE must be sandbox or mock.');
  }
  if (!stopping) start('cargo', ['run', '--package', 'nvbes-billing-service', '--', 'serve'], env);
}

if (import.meta.url === pathToFileURL(resolve(process.argv[1] ?? '')).href) {
  main().catch((error) => {
    console.error(error.message);
    process.exitCode = 1;
  });
}
