import { execFileSync } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';

export async function seedAuthContext(apiBaseUrl, rootDirectory) {
  const credentials = seededCredentialsFromEnv();
  let workspaceName = process.env.NVBES_SMOKE_AUTH_WORKSPACE ?? 'Smoke Contract Workspace';

  if (credentials.shouldRegister) {
    workspaceName = `Smoke Contract ${randomUUID().slice(0, 8)}`;
    await fetchJson(
      `${apiBaseUrl}/auth/register`,
      {
        method: 'POST',
        headers: jsonHeaders(),
        body: JSON.stringify({
          email: credentials.email,
          password: credentials.password,
          firstname: 'Smoke',
          lastname: 'Contract',
          username: `smoke_${randomUUID().slice(0, 8)}`,
          region: 'FR',
          workspace_name: workspaceName,
          ...(await fetchPowSolution(apiBaseUrl)),
        }),
      },
      [200, 409],
    );
  }

  prepareSeededAuthAccount(credentials, workspaceName, rootDirectory);
  const { json: identifier } = await fetchJson(`${apiBaseUrl}/auth/challenge/identifier`, {
    method: 'POST',
    headers: jsonHeaders(),
    body: JSON.stringify({
      email: credentials.email,
      ...(await fetchPowSolution(apiBaseUrl)),
    }),
  });
  const stateToken = identifier?.state_token;
  if (!stateToken) {
    throw new Error('seeded auth failed: /auth/challenge/identifier did not return state_token');
  }

  const { response: loginResponse, json: loginBody } = await fetchJson(
    `${apiBaseUrl}/auth/challenge/pwd`,
    {
      method: 'POST',
      headers: jsonHeaders(),
      body: JSON.stringify({
        state_token: stateToken,
        password: credentials.password,
      }),
    },
    [200, 202],
  );
  if (loginResponse.status === 202) {
    throw new Error('seeded auth failed: credentials require MFA, provide a non-MFA smoke account');
  }

  const token = extractSessionTokenFromSetCookies(extractSetCookies(loginResponse.headers));
  if (!token) {
    throw new Error('seeded auth failed: /auth/challenge/pwd did not set a session cookie');
  }

  const authHeaders = { Accept: '*/*', Authorization: `Bearer ${token}` };
  const [{ json: me }, { json: sessions }] = await Promise.all([
    fetchJson(`${apiBaseUrl}/auth/me`, { method: 'GET', headers: authHeaders }),
    fetchJson(`${apiBaseUrl}/auth/sessions`, {
      method: 'GET',
      headers: authHeaders,
    }),
  ]);
  if (!me?.user?.id) {
    throw new Error('seeded auth failed: /auth/me did not return user.id');
  }

  const currentSession = Array.isArray(sessions?.sessions)
    ? (sessions.sessions.find((entry) => entry.current) ?? sessions.sessions[0])
    : undefined;
  return {
    token,
    userId: me.user.id,
    workspaceId: me.current_workspace_id ?? undefined,
    sessionId: currentSession?.id ?? undefined,
    loginSummary: {
      email: credentials.email,
      workspaceId: me.current_workspace_id ?? null,
      sessionId: currentSession?.id ?? null,
      mfaEnabled: Boolean(loginBody?.user?.mfa_enabled),
    },
  };
}

async function fetchJson(url, init = {}, allowedStatuses = [200]) {
  let response;
  try {
    response = await fetch(url, { ...init, redirect: 'manual' });
  } catch (error) {
    const causeCode = error?.cause?.code ? ` (${error.cause.code})` : '';
    throw new Error(`fetch ${init.method ?? 'GET'} ${url} failed${causeCode}`);
  }
  if (!allowedStatuses.includes(response.status)) {
    throw new Error(`${init.method ?? 'GET'} ${url} returned unexpected status ${response.status}`);
  }
  if (response.status === 204) return { response, json: null };
  const payload = await response.text();
  if (!payload) return { response, json: null };
  try {
    return { response, json: JSON.parse(payload) };
  } catch {
    throw new Error(`${init.method ?? 'GET'} ${url} did not return valid JSON`);
  }
}

async function fetchPowSolution(apiBaseUrl) {
  let response;
  try {
    response = await fetch(`${apiBaseUrl}/auth/challenge/pow`, {
      method: 'GET',
      headers: { Accept: '*/*' },
      redirect: 'manual',
    });
  } catch {
    throw new Error('GET /auth/challenge/pow failed');
  }
  if (!response.ok) return {};
  const challenge = await response.json();
  if (
    !challenge ||
    typeof challenge !== 'object' ||
    !challenge.nonce ||
    !challenge.difficulty ||
    challenge.difficulty <= 0
  ) {
    return {};
  }
  return {
    pow_nonce: challenge.nonce,
    pow_solution: solvePow(challenge.nonce, challenge.difficulty),
  };
}

function solvePow(nonce, difficulty) {
  for (let attempt = 0; attempt < 5_000_000; attempt += 1) {
    const candidate = String(attempt);
    const hash = createHash('sha256').update(`${nonce}:${candidate}`).digest();
    if (leadingZeroBits(hash) >= difficulty) return candidate;
  }
  throw new Error('unable to solve the PoW challenge within the bounded budget');
}

function leadingZeroBits(buffer) {
  let count = 0;
  for (const byte of buffer.values()) {
    if (byte === 0) {
      count += 8;
      continue;
    }
    return count + Math.clz32(byte) - 24;
  }
  return count;
}

function extractSetCookies(headers) {
  if (typeof headers.getSetCookie === 'function') return headers.getSetCookie();
  const raw = headers.get('set-cookie');
  return raw ? raw.split(/,(?=[^;]+=)/u) : [];
}

function extractSessionTokenFromSetCookies(setCookies) {
  for (const entry of setCookies) {
    const match = entry.trim().match(/^(__Host-session|session|__Host-token|token)=([^;]+)/iu);
    if (match) return decodeURIComponent(match[2]);
  }
  return undefined;
}

function seededCredentialsFromEnv() {
  const email = process.env.NVBES_SMOKE_AUTH_EMAIL;
  const password = process.env.NVBES_SMOKE_AUTH_PASSWORD;
  if ((email && !password) || (!email && password)) {
    throw new Error('NVBES_SMOKE_AUTH_EMAIL and NVBES_SMOKE_AUTH_PASSWORD must both be set');
  }
  if (email && password) return { email, password, shouldRegister: false };
  const suffix = randomUUID();
  return {
    email: `smoke.contract.${suffix}@example.com`,
    password: `SmokeContract!${suffix.slice(0, 12)}`,
    shouldRegister: true,
  };
}

function prepareSeededAuthAccount(credentials, workspaceName, rootDirectory) {
  const databaseUrl = process.env.NVBES_DATABASE_URL;
  if (!databaseUrl) {
    throw new Error('NVBES_DATABASE_URL is required for seeded-auth smoke preparation');
  }
  try {
    execFileSync(
      'cargo',
      [
        'run',
        '-q',
        '-p',
        'nvbes-account-service',
        '--',
        '--prepare-beta-e2e-account',
        '--email',
        credentials.email,
        '--workspace-name',
        workspaceName,
      ],
      {
        cwd: rootDirectory,
        encoding: 'utf8',
        stdio: ['ignore', 'pipe', 'pipe'],
        env: {
          ...process.env,
          NVBES_BETA_SEED_PASSWORD: credentials.password,
          NVBES_ENV: process.env.NVBES_ENV ?? 'test',
          NVBES_DATABASE_URL: databaseUrl,
        },
      },
    );
  } catch {
    throw new Error('seeded auth account preparation failed');
  }
}

function jsonHeaders() {
  return {
    Accept: '*/*',
    'Content-Type': 'application/json',
    Origin: 'http://localhost:3001',
  };
}
