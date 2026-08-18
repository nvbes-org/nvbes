import { createHash, randomUUID } from 'node:crypto';
import { expect, type Page } from '@playwright/test';

import { requiredEnvironment } from './identity-journey.support';

const CLIENT_ID = 'account-web';

export async function completeHostedAuthorizationCodeJourney(
  page: Page,
  email: string,
  password: string,
) {
  const apiBaseUrl = requiredEnvironment('NVBES_API_BASE_URL');
  const redirectUri = new URL(
    '/oauth/callback',
    requiredEnvironment('NVBES_WEB_BASE_URL'),
  ).toString();
  const state = randomUUID();
  const nonce = randomUUID();
  const codeVerifier = `${randomUUID().replaceAll('-', '')}${randomUUID().replaceAll('-', '')}`;
  const codeChallenge = createHash('sha256').update(codeVerifier).digest('base64url');

  const parResponse = await page.request.post(`${apiBaseUrl}/oauth/par`, {
    form: {
      response_type: 'code',
      client_id: CLIENT_ID,
      redirect_uri: redirectUri,
      scope: 'openid profile email',
      audience: 'nvbes-account-service',
      state,
      nonce,
      code_challenge: codeChallenge,
      code_challenge_method: 'S256',
    },
  });
  const parPayload = record(await parResponse.json());
  expect(parResponse.status(), JSON.stringify(parPayload)).toBe(201);
  const requestUri = requiredString(parPayload, 'request_uri');

  const authorizeUrl = new URL('/oauth/authorize', apiBaseUrl);
  authorizeUrl.searchParams.set('response_type', 'code');
  authorizeUrl.searchParams.set('client_id', CLIENT_ID);
  authorizeUrl.searchParams.set('request_uri', requestUri);
  await page.goto(authorizeUrl.toString());
  await expect(page).toHaveURL(/\/login\?state_id=hosted_/u);

  await page.getByLabel('Email', { exact: true }).fill(email);
  await page.getByRole('button', { name: 'Continuer' }).click();
  await expect(page.getByLabel('Mot de passe', { exact: true })).toBeVisible();
  await page.getByLabel('Mot de passe', { exact: true }).fill(password);
  await page.getByRole('button', { name: 'Se connecter' }).click();
  await expect(page.getByRole('heading', { name: 'Account Web' })).toBeVisible();
  await expect(page.getByText('Authentification', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Autoriser' }).click();
  await page.waitForURL((url) => url.pathname === '/oauth/callback');

  const callbackUrl = new URL(page.url());
  expect(callbackUrl.searchParams.get('state')).toBe(state);
  const code = callbackUrl.searchParams.get('code');
  expect(code).toMatch(/^gxac_[a-f0-9]{32}$/u);
  if (!code) {
    throw new Error('OAuth callback did not contain an authorization code.');
  }

  const tokenForm = {
    grant_type: 'authorization_code',
    code,
    redirect_uri: redirectUri,
    client_id: CLIENT_ID,
    code_verifier: codeVerifier,
  };
  const tokenResponse = await page.request.post(`${apiBaseUrl}/oauth/token`, {
    form: tokenForm,
  });
  const tokenPayload = record(await tokenResponse.json());
  expect(tokenResponse.status(), JSON.stringify(tokenPayload)).toBe(200);
  const accessToken = requiredString(tokenPayload, 'access_token');
  expect(requiredString(tokenPayload, 'token_type').toLowerCase()).toBe('bearer');
  const accessClaims = jwtClaims(accessToken);
  expect(accessClaims.aud).toBe('nvbes-account-service');
  expect(accessClaims.client_id).toBe(CLIENT_ID);
  expect(requiredString(accessClaims, 'scope').split(/\s+/u).sort()).toEqual(
    ['openid', 'profile', 'email'].sort(),
  );

  const idClaims = jwtClaims(requiredString(tokenPayload, 'id_token'));
  expect(idClaims.aud).toBe(CLIENT_ID);
  expect(idClaims.nonce).toBe(nonce);
  expect(typeof idClaims.sub).toBe('string');

  const replayResponse = await page.request.post(`${apiBaseUrl}/oauth/token`, {
    form: tokenForm,
  });
  const replayPayload = record(await replayResponse.json());
  expect(replayResponse.status(), JSON.stringify(replayPayload)).toBe(400);
  expect(record(replayPayload.error).code).toBe('invalid_grant');
}

function record(value: unknown): Record<string, unknown> {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) {
    throw new Error('Identity returned a non-object OAuth response.');
  }
  return value as Record<string, unknown>;
}

function requiredString(value: Record<string, unknown>, key: string): string {
  const field = value[key];
  if (typeof field !== 'string' || !field) {
    throw new Error(`Identity OAuth response is missing ${key}.`);
  }
  return field;
}

function jwtClaims(token: string): Record<string, unknown> {
  const segments = token.split('.');
  if (segments.length !== 3 || !segments[1]) {
    throw new Error('Identity returned a malformed JWT.');
  }
  return record(JSON.parse(Buffer.from(segments[1], 'base64url').toString('utf8')));
}
