import { describe, expect, it, vi } from 'vite-plus/test';
import { LogoutController } from './logout.controller';

function setup() {
  const gateway = {
    load: vi.fn<() => Promise<string | null>>(async () => 'proof'),
    confirm: vi.fn(async (_csrf: string) => {}),
  };
  return { gateway, controller: new LogoutController(gateway) };
}
describe('Identity logout confirmation', () => {
  it('never revokes on load or cancellation', async () => {
    const { gateway, controller } = setup();
    await Promise.all([controller.start(), controller.start()]);
    expect(gateway.load).toHaveBeenCalledTimes(1);
    expect(controller.snapshot().stage).toBe('confirm');
    controller.cancel();
    await controller.confirm();
    expect(gateway.confirm).not.toHaveBeenCalled();
  });
  it('sends one explicit confirmation and reports success only after acknowledgement', async () => {
    const { gateway, controller } = setup();
    await controller.start();
    await Promise.all([controller.confirm(), controller.confirm()]);
    expect(gateway.confirm).toHaveBeenCalledExactlyOnceWith('proof');
    expect(controller.snapshot().stage).toBe('complete');
  });
  it('keeps an uncertain result distinct from a successful logout', async () => {
    const { gateway, controller } = setup();
    gateway.confirm.mockRejectedValue(new Error('timeout'));
    await controller.start();
    await controller.confirm();
    await controller.confirm();
    expect(gateway.confirm).toHaveBeenCalledTimes(1);
    expect(controller.snapshot().stage).toBe('failed');
  });
  it('distinguishes missing cookies from confirmed revocation', async () => {
    const { gateway, controller } = setup();
    gateway.load.mockResolvedValue(null);
    await controller.start();
    await controller.confirm();
    expect(controller.snapshot().stage).toBe('absent');
    expect(gateway.confirm).not.toHaveBeenCalled();
  });
  it('ignores a context arriving after page disposal', async () => {
    const { controller } = setup();
    const pending = controller.start();
    controller.dispose();
    await pending;
    expect(controller.snapshot().stage).toBe('cancelled');
  });
});
