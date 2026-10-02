import { afterEach, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import {
  PasswordRecoveryController,
  recoveryLink,
  type PasswordRecoveryGateway,
} from './password-recovery.controller';
import { PasswordRecoveryPage } from './password-recovery.page';

afterEach(cleanup);
function gateway(): PasswordRecoveryGateway {
  return {
    context: vi.fn().mockResolvedValue('csrf'),
    request: vi.fn().mockResolvedValue(undefined),
    reset: vi.fn().mockResolvedValue(undefined),
  };
}
it('acknowledges requests without exposing account eligibility or repeating submission', async () => {
  const api = gateway();
  const controller = new PasswordRecoveryController(api);
  await controller.start();
  await Promise.all([
    controller.request('someone@example.invalid'),
    controller.request('someone@example.invalid'),
  ]);
  expect(api.request).toHaveBeenCalledTimes(1);
  expect(controller.snapshot().stage).toBe('sent');
  expect(JSON.stringify(controller.snapshot())).not.toContain('someone');
});
it('clears password fields before dispatch and retains no token in visible state', async () => {
  const api = gateway();
  const controller = new PasswordRecoveryController(api, 'a'.repeat(43));
  await controller.start();
  render(<PasswordRecoveryPage controller={controller} />);
  const password = screen.getByLabelText<HTMLInputElement>('Nouveau mot de passe');
  const confirmation = screen.getByLabelText<HTMLInputElement>('Confirmer le mot de passe');
  fireEvent.change(password, { target: { value: 'New-private-password!' } });
  fireEvent.change(confirmation, { target: { value: 'New-private-password!' } });
  fireEvent.click(screen.getByRole('button', { name: 'Changer le mot de passe' }));
  expect(password.value).toBe('');
  expect(confirmation.value).toBe('');
  await waitFor(() => expect(controller.snapshot().stage).toBe('complete'));
  expect(api.reset).toHaveBeenCalledExactlyOnceWith(
    'csrf',
    'a'.repeat(43),
    'New-private-password!',
  );
  expect(JSON.stringify(controller.snapshot())).not.toContain('private-password');
  await controller.reset('Other-private-password!', 'Other-private-password!');
  expect(api.reset).toHaveBeenCalledTimes(1);
});
it('does not retry a failed reset or accept a late success after disposal', async () => {
  const api = gateway();
  let finish: (() => void) | undefined;
  api.reset = vi.fn(
    () =>
      new Promise<void>((resolve) => {
        finish = resolve;
      }),
  );
  const controller = new PasswordRecoveryController(api, 'a'.repeat(43));
  await controller.start();
  const pending = controller.reset('new-password!', 'new-password!');
  controller.dispose();
  finish?.();
  await pending;
  expect(controller.snapshot().stage).toBe('closed');
  await controller.reset('new-password!', 'new-password!');
  expect(api.reset).toHaveBeenCalledTimes(1);
  const failed = gateway();
  failed.reset = vi.fn().mockRejectedValue(new Error('network'));
  const retry = new PasswordRecoveryController(failed, 'a'.repeat(43));
  await retry.start();
  await retry.reset('new-password!', 'new-password!');
  await retry.reset('new-password!', 'new-password!');
  expect(failed.reset).toHaveBeenCalledTimes(1);
  expect(retry.snapshot().stage).toBe('closed');
});
it('rejects ambiguous links before loading context and mismatched passwords before sending', async () => {
  for (const suffix of ['?token=secret', '#token=a&token=b', '#token=bad', '#other=ignored']) {
    expect(recoveryLink(new URL('https://identity.example/password-recovery' + suffix))).toBe('');
  }
  const api = gateway();
  const invalid = new PasswordRecoveryController(api, '');
  await invalid.start();
  expect(api.context).not.toHaveBeenCalled();
  const controller = new PasswordRecoveryController(api, 'a'.repeat(43));
  await controller.start();
  await controller.reset('password-one!', 'password-two!');
  expect(api.reset).not.toHaveBeenCalled();
  expect(controller.snapshot().stage).toBe('reset');
});
