import { afterEach, beforeEach, expect, it, vi } from 'vite-plus/test';
import { AccountController } from './account.controller';
import type { AccountGateway } from './account.gateway';
import { watchAccountSession } from './account.revalidation';

beforeEach(() => vi.useFakeTimers());
afterEach(() => vi.useRealTimers());

function fixture() {
  const profile = {
    id: 'subject',
    displayName: 'Name',
    firstname: null,
    lastname: null,
    username: null,
    region: null,
  };
  const gateway = {
    initialize: vi.fn(async () => {}),
    authorize: vi.fn(async () => 'https://identity.example'),
    callback: vi.fn<AccountGateway['callback']>(async () => 'connected'),
    profile: vi.fn<AccountGateway['profile']>(async () => profile),
    expiration: () => 0,
    logoutRequest: () => ({ action: 'https://identity.example/oauth/end-session', fields: {} }),
    clear: vi.fn(),
  };
  const controller = new AccountController(gateway, vi.fn(), vi.fn());
  let active = true;
  const windowEvents = new EventTarget();
  const documentEvents = new EventTarget();
  const stop = watchAccountSession(controller, {
    windowEvents,
    documentEvents,
    active: () => active,
  });
  return {
    gateway,
    controller,
    stop,
    profile,
    windowEvents,
    documentEvents,
    active: (value: boolean) => {
      active = value;
    },
    start: () => controller.start(new URL('https://account.example/oauth/callback?code=c&state=s')),
  };
}

it('checks a continuously visible session at one-minute intervals after each response', async () => {
  const f = fixture();
  await f.start();
  await vi.advanceTimersByTimeAsync(59_999);
  expect(f.gateway.profile).toHaveBeenCalledTimes(1);
  await vi.advanceTimersByTimeAsync(1);
  expect(f.gateway.profile).toHaveBeenCalledTimes(2);
  await vi.advanceTimersByTimeAsync(60_000);
  expect(f.gateway.profile).toHaveBeenCalledTimes(3);
  f.stop();
});

it('coalesces timer and focus events and never overlaps a slow request', async () => {
  const f = fixture();
  await f.start();
  let done!: (value: typeof f.profile) => void;
  f.gateway.profile.mockImplementationOnce(
    () =>
      new Promise((resolve) => {
        done = resolve;
      }),
  );
  await vi.advanceTimersByTimeAsync(60_000);
  f.windowEvents.dispatchEvent(new Event('focus'));
  f.documentEvents.dispatchEvent(new Event('visibilitychange'));
  await vi.advanceTimersByTimeAsync(180_000);
  expect(f.gateway.profile).toHaveBeenCalledTimes(2);
  expect(f.controller.snapshot().profile).toBeNull();
  done(f.profile);
  await vi.advanceTimersByTimeAsync(59_999);
  expect(f.gateway.profile).toHaveBeenCalledTimes(2);
  await vi.advanceTimersByTimeAsync(1);
  expect(f.gateway.profile).toHaveBeenCalledTimes(3);
  f.stop();
});

it.each(['visibilitychange', 'offline'])(
  'pauses on %s and checks immediately on resume',
  async (event) => {
    const f = fixture();
    await f.start();
    f.active(false);
    const target = event === 'offline' ? f.windowEvents : f.documentEvents;
    target.dispatchEvent(new Event(event));
    await vi.advanceTimersByTimeAsync(600_000);
    expect(f.gateway.profile).toHaveBeenCalledTimes(1);
    f.active(true);
    target.dispatchEvent(new Event(event === 'offline' ? 'online' : event));
    await vi.advanceTimersByTimeAsync(0);
    expect(f.gateway.profile).toHaveBeenCalledTimes(2);
    f.stop();
  },
);

it('clears the profile and stops polling on a denial or network error', async () => {
  const f = fixture();
  await f.start();
  f.gateway.profile.mockRejectedValueOnce(new Error('unavailable'));
  await vi.advanceTimersByTimeAsync(60_000);
  expect(f.controller.snapshot().stage).toBe('error');
  expect(f.controller.snapshot().profile).toBeNull();
  expect(f.gateway.clear).toHaveBeenCalledOnce();
  await vi.advanceTimersByTimeAsync(600_000);
  expect(f.gateway.profile).toHaveBeenCalledTimes(2);
  f.stop();
});

it('does not check before login and cancels timers/listeners at page disposal', async () => {
  const f = fixture();
  await vi.advanceTimersByTimeAsync(600_000);
  expect(f.gateway.profile).not.toHaveBeenCalled();
  await f.start();
  f.stop();
  f.stop();
  f.controller.dispose();
  f.windowEvents.dispatchEvent(new Event('focus'));
  f.documentEvents.dispatchEvent(new Event('visibilitychange'));
  await vi.advanceTimersByTimeAsync(600_000);
  expect(f.gateway.profile).toHaveBeenCalledTimes(1);
  expect(vi.getTimerCount()).toBe(0);
});
