#!/usr/bin/env node

import { execFileSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import {
  buildPath,
  buildRequestInit,
  collectParameters,
  deref,
  isProtectedOperation,
  ROUTABLE_METHODS,
  shouldSkipOperation,
  shouldSmokeThroughProxy,
} from './lib/openapi-probe-plan.mjs';
import { assertOpenApiProbeResponse } from './lib/openapi-response-contract.mjs';
import { seedAuthContext } from './lib/openapi-seeded-auth.mjs';

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const ROOT_DIR = path.resolve(scriptDirectory, '..');
const SEEDED_AUTH_ENABLED = readEnvBoolean('NVBES_SMOKE_CONTRACT_SEEDED_AUTH');
const FORBID_REDIRECTS = readEnvBoolean('NVBES_SMOKE_FORBID_REDIRECTS');

function fail(message) {
  throw new Error(message);
}

function readEnvBoolean(name) {
  const value = process.env[name];
  return value ? ['1', 'true', 'yes', 'on'].includes(value.toLowerCase()) : false;
}

function normalizeUrl(value, name) {
  if (!value) fail(`missing required environment variable: ${name}`);
  let res = String(value);
  while (res.endsWith('/')) res = res.slice(0, -1);
  return res;
}

function loadOpenApiSpec() {
  if (process.env.NVBES_OPENAPI_SPEC_PATH) {
    const customPath = path.resolve(ROOT_DIR, process.env.NVBES_OPENAPI_SPEC_PATH);
    if (!existsSync(customPath)) {
      fail(`specified NVBES_OPENAPI_SPEC_PATH not found: ${customPath}`);
    }
    return JSON.parse(readFileSync(customPath, 'utf8'));
  }

  if (process.env.NVBES_OPENAPI_EXPORT_FROM_CARGO) {
    const stdout = execFileSync(
      'cargo',
      ['run', '-p', 'nvbes-account-service', '--', '--export-openapi'],
      {
        cwd: ROOT_DIR,
        encoding: 'utf8',
        maxBuffer: 20 * 1024 * 1024,
        env: process.env,
      },
    );
    return JSON.parse(stdout);
  }

  const defaultSpecPath = path.resolve(ROOT_DIR, 'libs/ts/identity-sdk-core/openapi.json');
  if (existsSync(defaultSpecPath)) {
    return JSON.parse(readFileSync(defaultSpecPath, 'utf8'));
  }

  fail(`OpenAPI specification not found at default path: ${defaultSpecPath}`);
}

async function probe(baseUrl, pathName, method, operation, spec, authContext, protectedOperation) {
  const url = `${baseUrl}${pathName}`;
  const { headers, body } = buildRequestInit(
    method,
    operation,
    spec,
    authContext,
    protectedOperation,
  );
  await new Promise((resolve) => setTimeout(resolve, 50));

  let response;
  try {
    response = await fetch(url, {
      method: method.toUpperCase(),
      headers,
      body,
      redirect: 'manual',
    });
  } catch (error) {
    const causeCode = error?.cause?.code ? ` (${error.cause.code})` : '';
    fail(`fetch ${method.toUpperCase()} ${url} failed${causeCode}`);
  }

  try {
    assertOpenApiProbeResponse({
      response,
      operation,
      operationLabel: `${method.toUpperCase()} ${url}`,
      forbidRedirects: FORBID_REDIRECTS,
      authenticatedProbe: Boolean(authContext && protectedOperation),
    });
  } catch (error) {
    await response.body?.cancel().catch(() => {});
    fail(error instanceof Error ? error.message : 'OpenAPI response contract failed');
  }
  await response.body?.cancel().catch(() => {});
}

async function main() {
  const apiBaseUrl = normalizeUrl(process.env.NVBES_API_BASE_URL, 'NVBES_API_BASE_URL');
  const webBaseUrl = normalizeUrl(process.env.NVBES_WEB_BASE_URL, 'NVBES_WEB_BASE_URL');
  const spec = loadOpenApiSpec();
  const paths = spec.paths ?? {};
  const globalSecurity = spec.security;
  const authContext = SEEDED_AUTH_ENABLED ? await seedAuthContext(apiBaseUrl, ROOT_DIR) : undefined;

  let apiChecks = 0;
  let proxyChecks = 0;
  const skipped = [];
  for (const [pathPattern, pathItem] of Object.entries(paths)) {
    for (const [method, rawOperation] of Object.entries(pathItem)) {
      if (!ROUTABLE_METHODS.includes(method)) continue;

      const operation = deref(rawOperation, spec);
      const skipReason = shouldSkipOperation(
        pathPattern,
        method,
        operation,
        globalSecurity,
        SEEDED_AUTH_ENABLED,
      );
      if (skipReason) {
        skipped.push(skipReason);
        continue;
      }

      const protectedOperation = isProtectedOperation(operation, globalSecurity);
      const resolvedPath = buildPath(
        pathPattern,
        collectParameters(pathItem, operation, spec),
        authContext,
        spec,
      );
      process.stderr.write(`#${apiChecks + 1}: ${method.toUpperCase()} ${resolvedPath}\n`);
      await probe(
        apiBaseUrl,
        resolvedPath,
        method,
        operation,
        spec,
        authContext,
        protectedOperation,
      );
      apiChecks += 1;

      if (shouldSmokeThroughProxy(pathPattern) && protectedOperation) {
        await probe(
          webBaseUrl,
          resolvedPath,
          method,
          operation,
          spec,
          authContext,
          protectedOperation,
        );
        proxyChecks += 1;
      }
    }
  }

  process.stdout.write(
    `OpenAPI smoke passed: ${apiChecks} direct checks, ${proxyChecks} proxied checks\n`,
  );
  process.stdout.write(
    `Mode: ${SEEDED_AUTH_ENABLED ? 'seeded-auth' : 'public-only'}${
      SEEDED_AUTH_ENABLED ? ` (seed ${authContext.loginSummary.email})` : ''
    }\n`,
  );
  if (SEEDED_AUTH_ENABLED && authContext?.loginSummary) {
    process.stdout.write(
      `Seeded auth context: workspace=${
        authContext.loginSummary.workspaceId ?? '<none>'
      }, session=${authContext.loginSummary.sessionId ?? '<none>'}\n`,
    );
  }
  if (skipped.length > 0) {
    process.stdout.write(`Skipped ${skipped.length} non-smokeable checks:\n`);
    for (const entry of skipped) process.stdout.write(`  - ${entry}\n`);
  }
}

main().catch((error) => {
  process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
  process.exit(1);
});
