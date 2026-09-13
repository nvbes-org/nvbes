import assert from 'node:assert/strict';
import test from 'node:test';

import { assertOpenApiProbeResponse } from '../../scripts/lib/openapi-response-contract.mjs';
import {
  buildPath,
  buildRequestInit,
  shouldSkipOperation,
} from '../../scripts/lib/openapi-probe-plan.mjs';

function response(status, contentType = 'application/json; charset=utf-8') {
  return {
    status,
    headers: new Headers(contentType ? { 'content-type': contentType } : {}),
  };
}

const operation = {
  responses: {
    200: {
      content: {
        'application/json': {},
      },
    },
    401: {
      content: {
        'application/json': {},
      },
    },
    '4XX': {
      content: {
        'application/problem+json': {},
      },
    },
  },
};

test('OpenAPI smoke accepts only a declared status and media type', () => {
  assert.doesNotThrow(() =>
    assertOpenApiProbeResponse({
      response: response(200),
      operation,
      operationLabel: 'GET /health',
      forbidRedirects: true,
      authenticatedProbe: false,
    }),
  );
  assert.throws(
    () =>
      assertOpenApiProbeResponse({
        response: response(404, 'text/html'),
        operation: { responses: { 200: {} } },
        operationLabel: 'GET /health',
        forbidRedirects: false,
        authenticatedProbe: false,
      }),
    /undeclared OpenAPI status 404/u,
  );
  assert.throws(
    () =>
      assertOpenApiProbeResponse({
        response: response(400, 'text/html'),
        operation,
        operationLabel: 'GET /resource',
        forbidRedirects: false,
        authenticatedProbe: false,
      }),
    /returned content-type text\/html/u,
  );
});

test('production smoke rejects redirects even if OpenAPI declares them', () => {
  assert.throws(
    () =>
      assertOpenApiProbeResponse({
        response: response(302, undefined),
        operation: { responses: { 302: {} } },
        operationLabel: 'GET /health',
        forbidRedirects: true,
        authenticatedProbe: false,
      }),
    /forbidden redirect 302/u,
  );
});

test('seeded authenticated probes reject authentication and authorization failures', () => {
  for (const status of [401, 403]) {
    assert.throws(
      () =>
        assertOpenApiProbeResponse({
          response: response(status),
          operation,
          operationLabel: 'GET /auth/me',
          forbidRedirects: false,
          authenticatedProbe: true,
        }),
      /rejected the seeded authenticated identity/u,
    );
  }
});

test('OpenAPI planner skips destructive and unseeded protected operations', () => {
  assert.match(
    shouldSkipOperation(
      '/auth/me/delete',
      'post',
      { security: [{ bearerAuth: [] }] },
      undefined,
      true,
    ),
    /destructive/u,
  );
  assert.match(
    shouldSkipOperation('/auth/me', 'get', { security: [{ bearerAuth: [] }] }, undefined, false),
    /protected/u,
  );
  assert.equal(
    shouldSkipOperation('/auth/me', 'patch', { security: [{ bearerAuth: [] }] }, undefined, true),
    null,
  );
});

test('OpenAPI planner binds authenticated IDs and bearer credentials', () => {
  const authContext = {
    sessionId: 'session-123',
    token: 'synthetic-token',
    userId: 'user-123',
    workspaceId: 'workspace-123',
  };
  assert.equal(
    buildPath(
      '/workspaces/{workspaceId}/sessions/{sessionId}',
      [
        { in: 'path', name: 'workspaceId' },
        { in: 'path', name: 'sessionId' },
      ],
      authContext,
      {},
    ),
    '/workspaces/workspace-123/sessions/session-123',
  );
  const request = buildRequestInit('get', {}, {}, authContext, true);
  assert.equal(request.headers.Authorization, 'Bearer synthetic-token');
});

test('OpenAPI probe loads and parses the canonical active specification', async () => {
  const { existsSync, readFileSync } = await import('node:fs');
  const path = await import('node:path');
  const specPath = path.resolve('libs/ts/identity-sdk-core/openapi.json');
  assert.ok(existsSync(specPath), 'canonical openapi.json exists');
  const spec = JSON.parse(readFileSync(specPath, 'utf8'));
  assert.ok(spec.paths, 'OpenAPI paths are present');
  assert.ok(
    spec.paths['/.well-known/openid-configuration'],
    'well-known openid configuration is present',
  );
  assert.ok(spec.paths['/oauth/token'], 'oauth token route is present');
});
