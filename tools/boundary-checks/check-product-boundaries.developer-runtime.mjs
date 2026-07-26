import { existsSync, readdirSync, readFileSync } from 'node:fs';

function read(path, errors) {
  if (!existsSync(path)) {
    errors.push(`${path}: required Developer boundary source is missing`);
    return '';
  }
  return readFileSync(path, 'utf8');
}

export function checkDeveloperRuntimeBoundary(errors) {
  const accountSource = 'apps/account-service/src';
  for (const file of readdirSync(accountSource)) {
    if (file.startsWith('identity.domains.developer.')) {
      errors.push(`${accountSource}/${file}: Account Service must not embed the Developer domain`);
    }
  }

  const accountModules = read(`${accountSource}/identity.domains.mod.rs`, errors);
  if (
    accountModules.includes('domains.developer') ||
    accountModules.includes('developer::router')
  ) {
    errors.push(
      `${accountSource}/identity.domains.mod.rs: Account Service must not mount Developer routes`,
    );
  }

  const accountOpenapiPath = 'apps/account-service/openapi.json';
  const accountOpenapi = read(accountOpenapiPath, errors);
  if (accountOpenapi.includes('"/developer/')) {
    errors.push(`${accountOpenapiPath}: Account OpenAPI must not expose Developer routes`);
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
