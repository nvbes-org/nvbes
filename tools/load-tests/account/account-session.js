import { check, group, sleep } from 'k6';
import http from 'k6/http';
import { profileOptions, rejectLoadTarget, thinkTimeSeconds } from './profiles.js';

const profile = __ENV.K6_PROFILE || 'smoke';
let rawBaseUrl = __ENV.ACCOUNT_SERVICE_BASE_URL || '';
while (rawBaseUrl.endsWith('/')) rawBaseUrl = rawBaseUrl.slice(0, -1);
const baseUrl = rawBaseUrl;
const cookieHeader = __ENV.ACCOUNT_TEST_COOKIE || '';
const authuser = __ENV.ACCOUNT_TEST_AUTHUSER || '0';

export const options = profileOptions(profile, __ENV);

export function setup() {
  if (!baseUrl || !cookieHeader) {
    throw new Error(
      'ACCOUNT_SERVICE_BASE_URL and ACCOUNT_TEST_COOKIE are required for session load tests.',
    );
  }

  rejectLoadTarget(
    baseUrl,
    __ENV.NVBES_TARGET_ENV,
    __ENV.ACCOUNT_LOAD_ALLOWED_ORIGINS,
    __ENV.ACCOUNT_PRODUCTION_DENIED_ORIGINS,
  );
}

export default function accountSessionJourney() {
  const headers = {
    Cookie: cookieHeader,
    'X-Auth-User': authuser,
  };

  group('authenticated account reads', () => {
    const responses = http.batch([
      [
        'GET',
        `${baseUrl}/auth/me?authuser=${encodeURIComponent(authuser)}`,
        null,
        {
          headers,
          tags: { endpoint: 'auth_me' },
          timeout: '5s',
        },
      ],
      [
        'GET',
        `${baseUrl}/auth/sessions?limit=25&authuser=${encodeURIComponent(authuser)}`,
        null,
        {
          headers,
          tags: { endpoint: 'auth_sessions' },
          timeout: '5s',
        },
      ],
      [
        'GET',
        `${baseUrl}/auth/accounts`,
        null,
        {
          headers,
          tags: { endpoint: 'auth_accounts' },
          timeout: '5s',
        },
      ],
    ]);

    check(responses[0], {
      'me returns 200': (response) => response.status === 200,
    });
    check(responses[1], {
      'sessions returns 200': (response) => response.status === 200,
    });
    check(responses[2], {
      'accounts returns 200': (response) => response.status === 200,
    });
  });

  sleep(thinkTimeSeconds(__ENV.ACCOUNT_K6_THINK_TIME_SECONDS, 1));
}
