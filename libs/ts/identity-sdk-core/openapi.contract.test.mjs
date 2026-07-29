import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import openapiTS, { astToString, COMMENT_HEADER } from 'openapi-typescript';

const packageRoot = new URL('./', import.meta.url);
const openapiUrl = new URL('openapi.json', packageRoot);
const generatedTypesUrl = new URL('src/types.gen.ts', packageRoot);
const serviceOpenapiUrl = new URL('../../../apps/account-service/openapi.json', packageRoot);
const methods = ['get', 'put', 'post', 'delete', 'patch', 'options', 'head', 'trace'];

async function readJson(url) {
  return JSON.parse(await readFile(url, 'utf8'));
}

test('service, SDK and optional runtime OpenAPI documents are identical', async () => {
  const sdk = await readJson(openapiUrl);
  assert.deepEqual(await readJson(serviceOpenapiUrl), sdk);

  if (process.env.ACCOUNT_RUNTIME_OPENAPI) {
    assert.deepEqual(JSON.parse(await readFile(process.env.ACCOUNT_RUNTIME_OPENAPI, 'utf8')), sdk);
  }
});

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

  assert.ok(operationCount >= 80, `expected broad Account API surface, got ${operationCount}`);
  const oauthClient = [{ oauthClientBasic: [] }, { oauthClientMtls: [] }];
  assert.deepEqual(document.paths['/oauth/introspect'].post.security, oauthClient);
  assert.deepEqual(document.paths['/oauth/revoke'].post.security, oauthClient);
});
