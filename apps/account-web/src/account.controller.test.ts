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
    logoutUrl: vi.fn(() => 'https://identity.example/logout'),
    clear: vi.fn(),
  };
  const navigate = vi.fn();
  return { gateway, navigate, controller: new AccountController(gateway, navigate) };
}
const callback = new URL('https://account.example/oauth/callback?code=x&state=s');
describe('Account document lifecycle', () => {
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
