import { existsSync, readdirSync, readFileSync } from 'node:fs';

function read(path, errors) {
  if (!existsSync(path)) {
    errors.push(`${path}: required Developer boundary source is missing`);
    return '';
  }
  return readFileSync(path, 'utf8');
}

export function checkDeveloperRuntimeBoundary(errors) {
  const identitySource = 'apps/identity-service/src';
  for (const file of readdirSync(identitySource)) {
    if (file.startsWith('identity.domains.developer.')) {
      errors.push(
        `${identitySource}/${file}: Identity Service must not embed the Developer domain`,
      );
    }
  }

  const identityModules = read(`${identitySource}/identity.domains.mod.rs`, errors);
  if (
    identityModules.includes('domains.developer') ||
    identityModules.includes('developer::router')
  ) {
    errors.push(
      `${identitySource}/identity.domains.mod.rs: Identity Service must not mount Developer routes`,
    );
  }

  const identityOpenapiPath = 'apps/identity-service/openapi.json';
  const identityOpenapi = read(identityOpenapiPath, errors);
  if (identityOpenapi.includes('"/developer/')) {
    errors.push(`${identityOpenapiPath}: Identity OpenAPI must not expose Developer routes`);
  }

  const developerOpenapiPath = 'apps/developer-service/openapi.json';
  const developerOpenapi = read(developerOpenapiPath, errors);
  for (const route of ['/developer/me', '/developer/apps', '/developer/console/context']) {
    if (!developerOpenapi.includes(`"${route}"`)) {
      errors.push(`${developerOpenapiPath}: Developer Service must expose ${route}`);
    }
  }

  const consoleApiPath = 'apps/console-web/src/developer.api.ts';
  const consoleApi = read(consoleApiPath, errors);
  if (!consoleApi.includes('developerHttpClient') || consoleApi.includes('identityHttpClient')) {
    errors.push(`${consoleApiPath}: Developer Console APIs must target Developer Service`);
  }
}
