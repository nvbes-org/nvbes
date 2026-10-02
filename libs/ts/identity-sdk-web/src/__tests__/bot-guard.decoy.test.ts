/**
 * @vitest-environment jsdom
 */
import { describe, expect, it, vi } from 'vite-plus/test';
import { createDecoyField, createDecoyLinks, mountDecoyField } from '../bot-guard.decoy';

describe('bot-guard.decoy', () => {
  it('creates a decoy honeypot field that starts empty and cleans up', () => {
    const decoy = createDecoyField();
    expect(decoy.wrapper).toBeInstanceOf(HTMLElement);
    expect(decoy.getValue()).toBe('');

    const input = decoy.wrapper.querySelector('input');
    expect(input).not.toBeNull();
    if (input) {
      input.value = 'bot-spam-url.com';
      expect(decoy.getValue()).toBe('bot-spam-url.com');
    }

    decoy.destroy();
  });

  it('mounts decoy field into container with delay', async () => {
    vi.useFakeTimers();
    const container = document.createElement('div');
    const mountPromise = mountDecoyField(container);

    vi.advanceTimersByTime(100);
    const field = await mountPromise;

    expect(container.children.length).toBeGreaterThan(0);
    expect(field.getValue()).toBe('');
    field.destroy();
    vi.useRealTimers();
  });

  it('tracks clicks on decoy links', () => {
    const container = document.createElement('div');
    const tracker = createDecoyLinks(container, 2);

    expect(tracker.wasClicked()).toBe(false);

    const firstLink = container.querySelector('a');
    expect(firstLink).not.toBeNull();

    firstLink?.click();
    expect(tracker.wasClicked()).toBe(true);

    tracker.destroy();
  });
});
