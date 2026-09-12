// @vitest-environment happy-dom
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vite-plus/test';
import { ClipboardButton } from './clipboard-button';

let root: Root;
let host: HTMLDivElement;
beforeEach(() => {
  vi.stubGlobal('IS_REACT_ACT_ENVIRONMENT', true);
  vi.useFakeTimers();
  host = document.createElement('div');
  document.body.append(host);
  root = createRoot(host);
});
afterEach(async () => {
  await act(async () => root.unmount());
  host.remove();
  vi.useRealTimers();
  vi.restoreAllMocks();
  Reflect.deleteProperty(document, 'execCommand');
  vi.unstubAllGlobals();
});
function button() {
  const result = host.querySelector('button');
  if (!result) throw new Error('Missing copy button');
  return result;
}

describe('clipboard feedback and cleanup', () => {
  it('copies the exact value, announces success and restores the label after the deadline', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    vi.stubGlobal('navigator', { clipboard: { writeText } });
    await act(async () => root.render(<ClipboardButton value="copy me" className="custom" />));
    expect(button().type).toBe('button');
    expect(button().getAttribute('aria-label')).toBe('Copy');
    expect(button().className).toContain('custom');
    expect(button().outerHTML).toMatchSnapshot('idle copy control');
    await act(async () => button().click());
    expect(writeText).toHaveBeenCalledExactlyOnceWith('copy me');
    expect(button().getAttribute('aria-label')).toBe('Copied');
    expect(button().title).toBe('Copied');
    expect(host.querySelector('[aria-live="polite"]')?.textContent).toBe('Copied');
    expect(button().outerHTML).toMatchSnapshot('successful copy control');
    await act(async () => vi.advanceTimersByTime(1999));
    expect(button().title).toBe('Copied');
    await act(async () => vi.advanceTimersByTime(1));
    expect(button().title).toBe('Copy');
    expect(vi.getTimerCount()).toBe(0);
  });

  it('uses custom labels and cancels a pending reset when unmounted', async () => {
    vi.stubGlobal('navigator', { clipboard: { writeText: vi.fn().mockResolvedValue(undefined) } });
    await act(async () =>
      root.render(<ClipboardButton value="x" label="Copier" copiedLabel="Copié" resetMs={500} />),
    );
    await act(async () => button().click());
    expect(button().title).toBe('Copié');
    await act(async () => vi.advanceTimersByTime(500));
    expect(button().title).toBe('Copier');
    await act(async () => button().click());
    await act(async () => root.render(null));
    expect(vi.getTimerCount()).toBe(0);
  });

  it.each(['absent', 'denied'])(
    'falls back after clipboard access is %s and removes the textarea',
    async (mode) => {
      vi.stubGlobal(
        'navigator',
        mode === 'absent'
          ? {}
          : { clipboard: { writeText: vi.fn().mockRejectedValue(new Error('denied')) } },
      );
      const copy = vi.fn(() => {
        const textarea = document.querySelector('textarea');
        expect(textarea?.value).toBe('fallback value');
        expect(textarea?.readOnly).toBe(true);
        expect(textarea?.style.position).toBe('fixed');
        expect(textarea?.style.opacity).toBe('0');
        expect(textarea?.selectionStart).toBe(0);
        expect(textarea?.selectionEnd).toBe('fallback value'.length);
        return true;
      });
      Object.defineProperty(document, 'execCommand', { configurable: true, value: copy });
      await act(async () => root.render(<ClipboardButton value="fallback value" />));
      await act(async () => button().click());
      expect(copy).toHaveBeenCalledExactlyOnceWith('copy');
      expect(button().title).toBe('Copied');
      expect(document.querySelector('textarea')).toBeNull();
    },
  );

  it.each(['false', 'throw'])(
    'announces failed fallback (%s) without leaving private text in the DOM',
    async (mode) => {
      vi.stubGlobal('navigator', {});
      const copy = vi.fn(() => {
        if (mode === 'throw') throw new Error('denied');
        return false;
      });
      Object.defineProperty(document, 'execCommand', { configurable: true, value: copy });
      await act(async () => root.render(<ClipboardButton value="private value" />));
      await act(async () => button().click());
      expect(button().title).toBe('Copy');
      expect(host.querySelector('[aria-live="polite"]')?.textContent).toBe('Copy failed');
      expect(button().outerHTML).toMatchSnapshot('failed copy control');
      expect(document.querySelector('textarea')).toBeNull();
      expect(vi.getTimerCount()).toBe(0);
    },
  );
});
