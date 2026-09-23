import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import openapiTS, { astToString, COMMENT_HEADER } from 'openapi-typescript';

const packageRoot = new URL('./', import.meta.url);
const openapiUrl = new URL('openapi.json', packageRoot);
const generatedTypesUrl = new URL('src/types.gen.ts', packageRoot);
const methods = ['get', 'put', 'post', 'delete', 'patch', 'options', 'head', 'trace'];

async function readJson(url) {
  return JSON.parse(await readFile(url, 'utf8'));
}

test('generated TypeScript is exactly synchronized with OpenAPI', async () => {
  const ast = await openapiTS(openapiUrl);
  const expected = `${COMMENT_HEADER}${astToString(ast)}`;
  assert.equal(await readFile(generatedTypesUrl, 'utf8'), expected);
});

test('every operation has a unique ID and an explicit valid security contract', async () => {
  const document = await readJson(openapiUrl);
  const schemes = document.components?.securitySchemes ?? {};
  const operationIds = new Set();
  let operationCount = 0;

  for (const [path, pathItem] of Object.entries(document.paths ?? {})) {
    for (const method of methods) {
      const operation = pathItem[method];
      if (!operation) continue;
      operationCount += 1;
      assert.equal(typeof operation.operationId, 'string', `${method} ${path} operationId`);
      assert.equal(operationIds.has(operation.operationId), false, operation.operationId);
      operationIds.add(operation.operationId);
      assert.ok(Array.isArray(operation.security), `${method} ${path} security`);
      for (const requirement of operation.security) {
        for (const scheme of Object.keys(requirement)) {
          assert.ok(schemes[scheme], `${method} ${path} references ${scheme}`);
        }
      }
    }
  }

  assert.ok(operationCount > 0, 'Account OpenAPI must expose operations');

  for (const path of [
    '/api/v1/profile',
    '/api/v1/preferences',
    '/api/v1/consents',
    '/api/v1/teams',
    '/api/v1/teams/join',
    '/api/v1/teams/{teamId}/leave',
    '/api/v1/privacy/exports',
    '/api/v1/closure',
  ]) {
    assert.ok(document.paths[path], `missing Account V1 path ${path}`);
  }

  for (const path of [
    '/oauth/authorize',
    '/oauth/token',
    '/auth/challenge/identifier',
    '/api/v1/privacy/gpc',
    '/api/v1/security/sessions',
  ]) {
    assert.equal(
      document.paths[path],
      undefined,
      `non-Account surface leaked into Account OpenAPI: ${path}`,
    );
  }

  const scopes = document.components.securitySchemes.identityBearer.flows.authorizationCode.scopes;
  for (const scope of ['account:read', 'account:write', 'account:export', 'account:close']) {
    assert.ok(scopes[scope], `missing Account scope ${scope}`);
  }
  assert.equal(scopes['teams:read'], undefined);
  assert.equal(scopes['account:legal:read'], undefined);
});
