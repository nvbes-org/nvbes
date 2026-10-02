import { afterEach, expect, it, vi } from 'vite-plus/test';
import { HostedIdentityError } from '@nvbes/identity-sdk-web/oauth';
import { FactorsController, factorsGateway } from './factors.controller';

afterEach(() => vi.useRealTimers());
function setup() {
  const gateway = factorsGateway('https://identity.example');
  const factors = {
    passkeys: [{ id: 'key', label: 'Key', createdAt: '2026-01-01', lastUsedAt: null }],
    totp: [{ id: 'totp', createdAt: '2026-01-01' }],
  };
  const list = vi.spyOn(gateway, 'list').mockResolvedValue(factors);
  const rename = vi.spyOn(gateway, 'rename').mockResolvedValue(undefined);
  const revoke = vi.spyOn(gateway, 'revoke').mockResolvedValue(undefined);
  const close = vi.fn();
  const controller = new FactorsController(
    gateway,
    'csrf',
    new Date(Date.now() + 60_000).toISOString(),
    close,
  );
  return { controller, list, rename, revoke, close, factors };
}

it('loads once and renames only an owned listed passkey with one mutation', async () => {
  const test = setup();
  await Promise.all([test.controller.start(), test.controller.start()]);
  expect(test.list).toHaveBeenCalledTimes(1);
  await test.controller.rename('unknown', 'name');
  expect(test.rename).not.toHaveBeenCalled();
  await Promise.all([test.controller.rename('key', 'New'), test.controller.rename('key', 'Other')]);
  expect(test.rename).toHaveBeenCalledExactlyOnceWith('csrf', 'key', 'New');
  expect(test.controller.snapshot().passkeys[0]?.label).toBe('New');
});

it.each(['passkey', 'totp'] as const)(
  'ends the interaction after confirmed %s revocation',
  async (kind) => {
    const test = setup();
    await test.controller.start();
    await test.controller.revoke(kind, kind === 'passkey' ? 'key' : 'totp');
    expect(test.close).toHaveBeenCalledExactlyOnceWith(true);
    expect(test.controller.snapshot().passkeys).toEqual([]);
    await test.controller.rename('key', 'name');
    expect(test.rename).not.toHaveBeenCalled();
  },
);

it('refuses removal of the last listed factor without a mutation', async () => {
  const test = setup();
  test.list.mockResolvedValue({ ...test.factors, totp: [] });
  await test.controller.start();
  await test.controller.revoke('passkey', 'key');
  expect(test.revoke).not.toHaveBeenCalled();
  expect(test.controller.snapshot().error).toContain('dernière');
});

it('respects a server refusal after a concurrent factor change', async () => {
  const test = setup();
  await test.controller.start();
  test.revoke.mockRejectedValue(new HostedIdentityError(400));
  await test.controller.revoke('passkey', 'key');
  expect(test.close).not.toHaveBeenCalled();
  expect(test.controller.snapshot().error).not.toBeNull();
  expect(test.revoke).toHaveBeenCalledTimes(1);
});

it('expires before mutation and never restores a list received after exit', async () => {
  vi.useFakeTimers();
  const test = setup();
  await test.controller.start();
  vi.advanceTimersByTime(60_000);
  await test.controller.rename('key', 'name');
  expect(test.rename).not.toHaveBeenCalled();
  expect(test.close).toHaveBeenCalledWith(false);
  const late = setup();
  let finish: (value: typeof late.factors) => void = () => {};
  late.list.mockReturnValue(
    new Promise((resolve) => {
      finish = resolve;
    }),
  );
  const pending = late.controller.start();
  late.controller.dispose();
  finish(late.factors);
  await pending;
  expect(late.controller.snapshot().passkeys).toEqual([]);
});

it('closes on uncertain mutation without retry or a success claim', async () => {
  const test = setup();
  await test.controller.start();
  test.revoke.mockRejectedValue(new TypeError('Network error'));
  await test.controller.revoke('totp', 'totp');
  await test.controller.revoke('totp', 'totp');
  expect(test.revoke).toHaveBeenCalledTimes(1);
  expect(test.close).toHaveBeenCalledExactlyOnceWith(false);
});
