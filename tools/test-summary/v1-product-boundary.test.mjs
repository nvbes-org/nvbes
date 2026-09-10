import assert from 'node:assert/strict';
import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { test } from 'node:test';

await test('Cloud and Enterprise remain archived, outside the active Cargo workspace', () => {
  const cargo = readFileSync('Cargo.toml', 'utf8');
  for (const domain of ['cloud', 'enterprise']) {
    assert(!cargo.includes(`"libs/rust/products/${domain}"`));
    assert(!existsSync(`libs/rust/products/${domain}/Cargo.toml`));
    assert(existsSync(`archive/libs/rust/products/${domain}/Cargo.toml`));
    assert(!existsSync(`apps/${domain}-service`));
    assert(!existsSync(`apps/${domain}-web`));
    assert(existsSync(`archive/apps/${domain}-service`));
  }
  for (const file of readdirSync('apps', { recursive: true, encoding: 'utf8' }).filter((name) =>
    name.endsWith('Cargo.toml'),
  )) {
    assert.doesNotMatch(
      readFileSync(`apps/${file}`, 'utf8'),
      /nvbes-product-(cloud|enterprise)|archive\//u,
      file,
    );
  }
});

await test('the active identity client cannot export or import Enterprise and federation runtime', () => {
  for (const file of readdirSync('libs/ts/identity-client/src')) {
    assert.doesNotMatch(file, /^(enterprise\.|federation\.|identity\.enterprise|combined\.)/u);
    if (!file.endsWith('.ts') || file.endsWith('.test.ts')) continue;
    assert.doesNotMatch(
      readFileSync(`libs/ts/identity-client/src/${file}`, 'utf8'),
      /from ['"].*(?:enterprise|federation|archive)\b/u,
      file,
    );
  }
  for (const file of [
    'enterprise.client.ts',
    'identity.enterprise-client.ts',
    'federation.schemas.ts',
  ]) {
    assert(existsSync(`archive/libs/ts/identity-client/src/${file}`));
  }
  assert(readFileSync('.nxignore', 'utf8').split('\n').includes('archive/**'));
});

await test('root commands and fuzzing do not reconnect archived product runtimes', () => {
  const pkg = JSON.parse(readFileSync('package.json', 'utf8'));
  for (const command of Object.values(pkg.scripts)) {
    assert.doesNotMatch(
      command,
      /scripts\/(?:db-migrate|migrate-staging|generate-openapi|test-identity-e2e-local)\.sh|upload_inputs/u,
    );
  }
  assert.doesNotMatch(
    readFileSync('fuzz/Cargo.toml', 'utf8'),
    /nvbes-cloud-service|upload_inputs/u,
  );
  assert.doesNotMatch(
    readFileSync('scripts/dev-hot.sh', 'utf8'),
    /products\/(?:enterprise|cloud)/u,
  );
});
