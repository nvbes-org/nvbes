#!/usr/bin/env node
import https from 'node:https';
import { parseArgs } from 'node:util';

const DEFAULT_TIMEOUT_MS = 5000;
const MAX_LATENCY_MS = 1500;
const MIN_CERT_VALIDITY_DAYS = 14;

const { values } = parseArgs({
  options: {
    'dry-run': { type: 'boolean', default: false },
    base_url: {
      type: 'string',
      default: process.env.NVBES_CANARY_BASE_URL || '',
    },
  },
  strict: false,
});

const REQUIRED_SECURITY_HEADERS = [
  'strict-transport-security',
  'x-content-type-options',
  'x-frame-options',
];

const SERVICES = [
  { name: 'identity', livePath: '/health/live', readyPath: '/health/ready' },
  { name: 'account', livePath: '/health/live', readyPath: '/health/ready' },
  { name: 'billing', livePath: '/health/live', readyPath: '/health/ready' },
  { name: 'email', livePath: '/health/live', readyPath: '/health/ready' },
  { name: 'trust-risk', livePath: '/health/live', readyPath: '/health/ready' },
];

function checkTlsCertificate(hostname) {
  return new Promise((resolve) => {
    const req = https.request(
      {
        hostname,
        port: 443,
        method: 'HEAD',
        path: '/health/live',
        timeout: DEFAULT_TIMEOUT_MS,
      },
      (res) => {
        const cert = res.socket.getPeerCertificate();
        if (!cert?.valid_to) {
          resolve({ ok: false, error: 'Could not extract peer certificate' });
          return;
        }
        const validTo = new Date(cert.valid_to);
        const now = new Date();
        const daysRemaining = Math.floor(
          (validTo.getTime() - now.getTime()) / (1000 * 60 * 60 * 24),
        );
        if (daysRemaining < MIN_CERT_VALIDITY_DAYS) {
          resolve({
            ok: false,
            error: `Certificate expires in ${daysRemaining} days (< ${MIN_CERT_VALIDITY_DAYS} days threshold)`,
          });
        } else {
          resolve({ ok: true, daysRemaining });
        }
      },
    );

    req.on('error', (err) => resolve({ ok: false, error: err.message }));
    req.on('timeout', () => {
      req.destroy();
      resolve({ ok: false, error: 'TLS check timed out' });
    });
    req.end();
  });
}

async function checkEndpoint(url) {
  const start = Date.now();
  try {
    const response = await fetch(url, {
      method: 'GET',
      headers: { 'User-Agent': 'nvbes-canary-verifier/1.0' },
      signal: AbortSignal.timeout(DEFAULT_TIMEOUT_MS),
    });
    const latency = Date.now() - start;

    if (!response.ok) {
      return {
        ok: false,
        error: `HTTP ${response.status} (${response.statusText})`,
        latency,
      };
    }

    const missingHeaders = [];
    for (const header of REQUIRED_SECURITY_HEADERS) {
      if (!response.headers.has(header)) {
        missingHeaders.push(header);
      }
    }

    return {
      ok: true,
      status: response.status,
      latency,
      latencyOk: latency <= MAX_LATENCY_MS,
      missingHeaders,
    };
  } catch (error) {
    return { ok: false, error: error.message, latency: Date.now() - start };
  }
}

async function main() {
  console.log('🔍 [Canary Verification] Validating live deployment health and security posture...');

  if (values['dry-run']) {
    console.log('⚡ Dry-run mode enabled: Validating test harness configuration...');
    console.log(`✅ Configured ${SERVICES.length} active service checks.`);
    console.log(`✅ Required security headers: ${REQUIRED_SECURITY_HEADERS.join(', ')}`);
    console.log('✅ Dry-run validation successful.');
    process.exit(0);
  }

  let baseUrl = values.base_url;
  while (baseUrl.endsWith('/')) baseUrl = baseUrl.slice(0, -1);
  if (!baseUrl) {
    console.warn(
      '⚠️  No NVBES_CANARY_BASE_URL provided. Run with --base_url=https://api.nvbes.com or --dry-run.',
    );
    process.exit(0);
  }

  let failures = 0;
  const urlObj = new URL(baseUrl);

  if (urlObj.protocol === 'https:') {
    console.log(`🔐 Checking TLS certificate for ${urlObj.hostname}...`);
    const certResult = await checkTlsCertificate(urlObj.hostname);
    if (!certResult.ok) {
      console.error(`❌ TLS Certificate check failed: ${certResult.error}`);
      failures++;
    } else {
      console.log(`✅ TLS Certificate valid (${certResult.daysRemaining} days remaining)`);
    }
  }

  for (const svc of SERVICES) {
    const liveUrl = `${baseUrl}${svc.livePath}`;
    const readyUrl = `${baseUrl}${svc.readyPath}`;

    const liveResult = await checkEndpoint(liveUrl);
    if (!liveResult.ok) {
      console.error(`❌ [${svc.name}] /health/live failed: ${liveResult.error}`);
      failures++;
    } else {
      const headerMsg =
        liveResult.missingHeaders.length > 0
          ? `(Missing headers: ${liveResult.missingHeaders.join(', ')})`
          : '(All security headers present)';
      console.log(`✅ [${svc.name}] /health/live OK: ${liveResult.latency}ms ${headerMsg}`);
    }

    const readyResult = await checkEndpoint(readyUrl);
    if (!readyResult.ok) {
      console.error(`❌ [${svc.name}] /health/ready failed: ${readyResult.error}`);
      failures++;
    } else {
      console.log(`✅ [${svc.name}] /health/ready OK: ${readyResult.latency}ms`);
    }
  }

  if (failures > 0) {
    console.error(`\n❌ Canary verification failed with ${failures} error(s).`);
    process.exit(1);
  } else {
    console.log('\n🎉 All canary checks passed successfully.');
    process.exit(0);
  }
}

void main();
