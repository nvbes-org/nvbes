import { describe, expect, it, vi } from 'vite-plus/test';
import { AccountController } from './account.controller';
import type { AccountGateway } from './account.gateway';

function setup() {
  const profile = {
    id: 'subject',
    displayName: 'Alice',
    firstname: null,
    lastname: null,
    username: null,
    region: null,
  };
  const gateway = {
    initialize: vi.fn(async () => {}),
    authorize: vi.fn(async () => 'https://identity.example/oauth/authorize'),
    callback: vi.fn<AccountGateway['callback']>(async () => 'connected'),
    profile: vi.fn<AccountGateway['profile']>(async () => profile),
    expiration: vi.fn(() => 0),
    logoutRequest: vi.fn(() => ({
      action: 'https://identity.example/oauth/end-session',
      fields: { state: 'test' },
    })),
    clear: vi.fn(),
  };
  const navigate = vi.fn();
  const submitLogout = vi.fn();
  return {
    gateway,
    navigate,
    submitLogout,
    controller: new AccountController(gateway, navigate, submitLogout),
  };
}
const callback = new URL('https://account.example/oauth/callback?code=x&state=s');
describe('Account document lifecycle', () => {
  it('clears local credentials before submitting a single RP logout', async () => {
    const { controller, gateway, submitLogout, navigate } = setup();
    await controller.start(callback);
    controller.logout();
    controller.logout();
    expect(gateway.clear).toHaveBeenCalledTimes(1);
    expect(submitLogout).toHaveBeenCalledExactlyOnceWith(
      gateway.logoutRequest.mock.results[0].value,
    );
    expect(navigate).not.toHaveBeenCalled();
    expect(controller.snapshot().profile).toBeNull();
  });
  it('only announces logout after a verified callback', async () => {
    const good = setup();
    await good.controller.start(undefined, 'complete');
    expect(good.controller.snapshot().message).toContain('déconnecté');
    expect(good.gateway.profile).not.toHaveBeenCalled();
    const bad = setup();
    await bad.controller.start(undefined, 'invalid');
    expect(bad.controller.snapshot().stage).toBe('error');
    expect(bad.controller.snapshot().message).not.toContain('déconnecté');
  });
  it('hides the profile during one coalesced foreground verification', async () => {
    const { controller, gateway } = setup();
    await controller.start(callback);
    const next = {
      id: 'subject',
      displayName: 'Updated',
      firstname: null,
      lastname: null,
      username: null,
      region: null,
    };
    let resolve!: (profile: typeof next) => void;
    gateway.profile.mockImplementationOnce(
      () =>
        new Promise((done) => {
          resolve = done;
        }),
    );
    const first = controller.revalidate();
    const second = controller.revalidate();
    expect(controller.snapshot()).toEqual({ stage: 'checking', profile: null, message: null });
    expect(gateway.profile).toHaveBeenCalledTimes(2);
    resolve(next);
    await Promise.all([first, second]);
    expect(controller.snapshot().profile?.displayName).toBe('Updated');
  });
  it('clears credentials on denied or unavailable foreground verification without retry', async () => {
    const { controller, gateway } = setup();
    await controller.start(callback);
    gateway.profile.mockRejectedValue(new Error('resource refused access'));
    await controller.revalidate();
    await controller.revalidate();
    expect(controller.snapshot().stage).toBe('error');
    expect(controller.snapshot().profile).toBeNull();
    expect(gateway.clear).toHaveBeenCalledTimes(1);
    expect(gateway.profile).toHaveBeenCalledTimes(2);
  });
  it('does not refresh a profile before login or after local closure', async () => {
    const { controller, gateway } = setup();
    await controller.revalidate();
    await controller.start();
    await controller.revalidate();
    controller.close();
    await controller.revalidate();
    expect(gateway.profile).not.toHaveBeenCalled();
  });
  it('ignores a foreground response after the page has closed', async () => {
    const { controller, gateway } = setup();
    await controller.start(callback);
    const work = controller.revalidate();
    controller.close();
    await work;
    expect(controller.snapshot().stage).toBe('closed');
    expect(controller.snapshot().profile).toBeNull();
    expect(gateway.clear).toHaveBeenCalled();
  });
  it('removes the profile and credentials at expiry', async () => {
    vi.useFakeTimers();
    try {
      const { controller, gateway } = setup();
      const expiry = Date.now() + 1000;
      gateway.expiration.mockReturnValue(expiry);
      await controller.start(callback);
      expect(controller.snapshot().stage).toBe('profile');
      await vi.advanceTimersByTimeAsync(1000);
      expect(controller.snapshot().stage).toBe('closed');
      expect(controller.snapshot().profile).toBeNull();
      expect(gateway.clear).toHaveBeenCalled();
    } finally {
      vi.useRealTimers();
    }
  });
  it('exchanges one callback even with repeated initialization', async () => {
    const { controller, gateway } = setup();
    await Promise.all([controller.start(callback), controller.start(callback)]);
    expect(gateway.callback).toHaveBeenCalledTimes(1);
    expect(gateway.profile).toHaveBeenCalledTimes(1);
    expect(controller.snapshot().profile?.displayName).toBe('Alice');
  });
  it('serializes repeated login clicks', async () => {
    const { controller, gateway, navigate } = setup();
    await controller.start();
    await Promise.all([controller.login(), controller.login()]);
    expect(gateway.authorize).toHaveBeenCalledTimes(1);
    expect(navigate).toHaveBeenCalledTimes(1);
  });
  it('does not fetch a profile after consent denial', async () => {
    const { controller, gateway } = setup();
    gateway.callback.mockResolvedValue('denied');
    await controller.start(callback);
    expect(gateway.profile).not.toHaveBeenCalled();
    expect(controller.snapshot().stage).toBe('ready');
  });
  it('does not retry an uncertain code exchange or expose backend messages', async () => {
    const { controller, gateway } = setup();
    gateway.callback.mockRejectedValue(new Error('secret server response'));
    await controller.start(callback);
    await controller.start(callback);
    await controller.login();
    expect(gateway.callback).toHaveBeenCalledTimes(1);
    expect(gateway.clear).toHaveBeenCalled();
    expect(controller.snapshot().message).not.toContain('secret');
    expect(controller.snapshot().stage).toBe('error');
  });
  it('clears the profile on close and never restores a late response', async () => {
    const { controller, gateway } = setup();
    let resolve!: (value: Awaited<ReturnType<AccountGateway['profile']>>) => void;
    gateway.profile.mockImplementation(
      () =>
        new Promise((done) => {
          resolve = done;
        }),
    );
    const work = controller.start(callback);
    await vi.waitFor(() => expect(gateway.profile).toHaveBeenCalled());
    controller.close();
    resolve({
      id: 's',
      displayName: 'Late',
      firstname: null,
      lastname: null,
      username: null,
      region: null,
    });
    await work;
    expect(controller.snapshot().stage).toBe('closed');
    expect(controller.snapshot().profile).toBeNull();
    expect(gateway.clear).toHaveBeenCalled();
  });
  it('cannot redirect after the document was disposed', async () => {
    const { controller, navigate } = setup();
    await controller.start();
    const work = controller.login();
    controller.dispose();
    await work;
    expect(navigate).not.toHaveBeenCalled();
  });
});
