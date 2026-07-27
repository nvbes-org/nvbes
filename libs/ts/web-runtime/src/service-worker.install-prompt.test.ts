import { afterEach, describe, expect, it, vi } from 'vite-plus/test';

type InstallPromptEvent = Event & {
  prompt: ReturnType<typeof vi.fn>;
  userChoice: Promise<{ outcome: 'accepted' | 'dismissed' }>;
};

function createInstallPromptEvent(outcome: 'accepted' | 'dismissed' = 'accepted') {
  const event = new Event('beforeinstallprompt', { cancelable: true }) as InstallPromptEvent;
  event.prompt = vi.fn(async () => undefined);
  event.userChoice = Promise.resolve({ outcome });
  return event;
}

async function installRuntimeWindow() {
  vi.resetModules();
  const runtimeWindow = new EventTarget();
  vi.stubGlobal('window', runtimeWindow);
  const runtime = await import('./service-worker');
  return { runtime, runtimeWindow };
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('PWA install prompt', () => {
  it('leaves the browser install banner enabled when no custom prompt is registered', async () => {
    const { runtimeWindow } = await installRuntimeWindow();
    const event = createInstallPromptEvent();

    runtimeWindow.dispatchEvent(event);

    expect(event.defaultPrevented).toBe(false);
    expect(event.prompt).not.toHaveBeenCalled();
  });

  it('defers the browser prompt when a custom install action is registered', async () => {
    const { runtime, runtimeWindow } = await installRuntimeWindow();
    let prompt: (() => Promise<void>) | undefined;
    runtime.onInstallReady((readyPrompt) => {
      prompt = readyPrompt;
    });
    const event = createInstallPromptEvent();

    runtimeWindow.dispatchEvent(event);

    expect(event.defaultPrevented).toBe(true);
    expect(prompt).toBeDefined();

    await prompt?.();

    expect(event.prompt).toHaveBeenCalledOnce();
  });
});
