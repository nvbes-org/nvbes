import { afterEach, expect, it, vi } from 'vite-plus/test';
import { HostedIdentityError, WebauthnBrowserError } from '@nvbes/identity-sdk-web/oauth';
import { RecoveryController, recoveryGateway } from './recovery.controller';

afterEach(() => vi.useRealTimers());
function setup() {
  const gateway = recoveryGateway('https://identity.example');
  const proof = { csrfToken: 'synthetic', expiresAt: new Date(Date.now() + 60_000).toISOString() };
  const resume = vi.spyOn(gateway, 'resume').mockResolvedValue(proof);
  const complete = vi
    .spyOn(gateway, 'complete')
    .mockResolvedValue({ credentialId: 'id', mustReauthenticate: true });
  const cancel = vi.spyOn(gateway, 'cancel').mockResolvedValue(undefined);
  return { controller: new RecoveryController(gateway), resume, complete, cancel, proof };
}

it('bootstraps once, serializes completion and does not create an ordinary session', async () => {
  const test = setup();
  await Promise.all([test.controller.start(), test.controller.start()]);
  expect(test.resume).toHaveBeenCalledTimes(1);
  await Promise.all([test.controller.complete('key'), test.controller.complete('key')]);
  expect(test.complete).toHaveBeenCalledExactlyOnceWith(test.proof, 'key');
  expect(test.controller.snapshot().stage).toBe('complete');
  expect(test.controller.snapshot().expiresAt).toBeNull();
  await test.controller.cancel();
  expect(test.cancel).not.toHaveBeenCalled();
});

it('preserves recovery after browser cancellation but never retries automatically', async () => {
  const test = setup();
  await test.controller.start();
  test.complete.mockRejectedValueOnce(
    new WebauthnBrowserError('webauthn_not_allowed', 'cancelled'),
  );
  await test.controller.complete('key');
  expect(test.complete).toHaveBeenCalledTimes(1);
  expect(test.controller.snapshot().stage).toBe('ready');
  await test.controller.complete('key');
  expect(test.controller.snapshot().stage).toBe('complete');
});

it('closes on ambiguous network outcome and refuses another mutation', async () => {
  const test = setup();
  await test.controller.start();
  test.complete.mockRejectedValue(new TypeError('Lost response'));
  await test.controller.complete('key');
  await test.controller.complete('key');
  expect(test.complete).toHaveBeenCalledTimes(1);
  expect(test.controller.snapshot().stage).toBe('closed');
});

it('expires the recovery before any mutation', async () => {
  vi.useFakeTimers();
  const test = setup();
  await test.controller.start();
  vi.advanceTimersByTime(60_000);
  await test.controller.complete('key');
  expect(test.complete).not.toHaveBeenCalled();
  expect(test.controller.snapshot().stage).toBe('closed');
});

it('does not restore a late bootstrap after page exit', async () => {
  const test = setup();
  let resolve: (proof: typeof test.proof) => void = () => {};
  test.resume.mockReturnValue(
    new Promise((done) => {
      resolve = done;
    }),
  );
  const pending = test.controller.start();
  test.controller.dispose();
  resolve(test.proof);
  await pending;
  await test.controller.complete('key');
  expect(test.complete).not.toHaveBeenCalled();
  expect(test.controller.snapshot().stage).toBe('closed');
});

it('cancels using only the recovery proof and treats rejected sessions as closed', async () => {
  const test = setup();
  await test.controller.start();
  await test.controller.cancel();
  expect(test.cancel).toHaveBeenCalledExactlyOnceWith(test.proof);
  expect(test.controller.snapshot().stage).toBe('cancelled');
  const invalid = setup();
  invalid.resume.mockRejectedValue(new HostedIdentityError(401));
  await invalid.controller.start();
  expect(invalid.controller.snapshot().stage).toBe('closed');
});
