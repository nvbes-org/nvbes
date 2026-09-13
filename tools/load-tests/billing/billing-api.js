import { check, group, sleep } from 'k6';
import http from 'k6/http';
import { profileOptions, rejectLoadTarget, thinkTimeSeconds } from '../account/profiles.js';

const profile = __ENV.K6_PROFILE || 'smoke';
const baseUrl = normalizedTarget(
  __ENV.BILLING_SERVICE_BASE_URL || 'http://host.docker.internal:4002',
);

export const options = profileOptions(profile, __ENV);

export function setup() {
  rejectLoadTarget(
    baseUrl,
    __ENV.NVBES_TARGET_ENV,
    __ENV.BILLING_LOAD_ALLOWED_ORIGINS,
    __ENV.BILLING_PRODUCTION_DENIED_ORIGINS,
  );

  const response = http.get(`${baseUrl}/health/live`, {
    tags: { endpoint: 'health_live', operation: 'setup' },
    timeout: '5s',
  });
  if (response.status !== 200) {
    throw new Error(`Billing service readiness failed with status ${response.status}`);
  }
}

export default function billingPublicJourney() {
  const journey = __ITER % 3;

  if (journey === 0) {
    healthLive(baseUrl);
  } else if (journey === 1) {
    healthReady(baseUrl);
  } else {
    metrics(baseUrl);
  }

  sleep(thinkTimeSeconds(__ENV.BILLING_K6_THINK_TIME_SECONDS, 0.2));
}

function healthLive(target) {
  group('health live', () => {
    const response = http.get(`${target}/health/live`, {
      tags: { endpoint: 'health_live' },
      timeout: '5s',
    });
    check(response, {
      'health/live returns 200': (result) => result.status === 200,
      'health/live reports alive': (result) => json(result)?.status === 'alive',
      'health/live identifies service': (result) =>
        json(result)?.service === 'nvbes-billing-service',
    });
  });
}

function healthReady(target) {
  group('health ready', () => {
    const response = http.get(`${target}/health/ready`, {
      tags: { endpoint: 'health_ready' },
      timeout: '5s',
    });
    check(response, {
      'health/ready returns 200 or 503': (result) => result.status === 200 || result.status === 503,
      'health/ready identifies service': (result) =>
        json(result)?.service === 'nvbes-billing-service',
    });
  });
}

function metrics(target) {
  group('metrics', () => {
    const response = http.get(`${target}/metrics`, {
      tags: { endpoint: 'metrics' },
      timeout: '5s',
    });
    check(response, {
      'metrics returns 200': (result) => result.status === 200,
    });
  });
}

function json(response) {
  try {
    return response.json();
  } catch {
    return undefined;
  }
}

function normalizedTarget(value) {
  return value.replace(/\/+$/u, '');
}
