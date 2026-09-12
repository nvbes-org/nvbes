import { afterEach, expect, it, vi } from 'vite-plus/test';

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

async function setup(supported = true) {
  vi.resetModules();
  const get = vi.fn();
  const store = vi.fn().mockResolvedValue(undefined);
  const preventSilentAccess = vi.fn().mockResolvedValue(undefined);
  class PasswordCredential {
    readonly type = 'password';
    constructor(readonly data: PasswordCredentialData) {}
  }
  vi.stubGlobal('window', supported ? { PasswordCredential } : {});
  vi.stubGlobal('navigator', supported ? { credentials: { get, store, preventSilentAccess } } : {});
  return { api: await import('./credential-management'), get, store, preventSilentAccess };
}
it('safely handles unsupported and server environments', async () => {
  const { api } = await setup(false);
  expect(api.isCredentialManagementSupported()).toBe(false);
  await expect(api.getStoredPasswordCredential()).resolves.toBeNull();
  await expect(api.storePasswordCredential('id', 'synthetic')).resolves.toBeUndefined();
  await expect(api.preventAutoSignIn()).resolves.toBeUndefined();
  vi.stubGlobal('window', undefined);
  vi.stubGlobal('navigator', undefined);
  expect(api.isCredentialManagementSupported()).toBe(false);
  await expect(api.preventAutoSignIn()).resolves.toBeUndefined();
});
it('reads password credentials silently and ignores other kinds or denial', async () => {
  const { api, get } = await setup();
  expect(api.isCredentialManagementSupported()).toBe(true);
  get.mockResolvedValue({ type: 'password', id: 'person', password: 'synthetic', name: 'Person' });
  await expect(api.getStoredPasswordCredential()).resolves.toEqual({
    id: 'person',
    password: 'synthetic',
    name: 'Person',
  });
  expect(get).toHaveBeenCalledExactlyOnceWith({ password: true, mediation: 'silent' });
  for (const credential of [null, { type: 'public-key' }]) {
    get.mockResolvedValue(credential);
    await expect(api.getStoredPasswordCredential()).resolves.toBeNull();
  }
  get.mockRejectedValue(new Error('denied'));
  await expect(api.getStoredPasswordCredential()).resolves.toBeNull();
});
it('stores the supplied credential and treats browser refusal as best effort', async () => {
  const { api, store, preventSilentAccess } = await setup();
  await api.storePasswordCredential('person', 'synthetic', 'Person');
  expect(store).toHaveBeenCalledWith(
    expect.objectContaining({ data: { id: 'person', password: 'synthetic', name: 'Person' } }),
  );
  await api.preventAutoSignIn();
  expect(preventSilentAccess).toHaveBeenCalledExactlyOnceWith();
  store.mockRejectedValue(new Error('denied'));
  preventSilentAccess.mockRejectedValue(new Error('denied'));
  await expect(api.storePasswordCredential('person', 'synthetic')).resolves.toBeUndefined();
  await expect(api.preventAutoSignIn()).resolves.toBeUndefined();
});
