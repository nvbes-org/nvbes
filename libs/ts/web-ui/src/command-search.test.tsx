// @vitest-environment happy-dom
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vite-plus/test';
import { CommandSearch } from './command-search';

const tokens = [
  { key: 'owner', description: 'Owner', example: 'owner:me' },
  { key: 'status', description: 'Status', example: 'status:open' },
];
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
  vi.unstubAllGlobals();
});
function input() {
  const result = host.querySelector('input');
  if (!result) throw new Error('Missing search field');
  return result;
}

describe('command suggestions', () => {
  it.each([
    ['', 2, 'owner:me '],
    ['Ow', 1, 'owner:me '],
    ['hello st', 1, 'hello status:open '],
    ['owner:me ', 2, 'owner:me owner:me '],
    ['status:open', 2, 'status:open owner:me '],
  ])('offers and inserts tokens for %j', async (value, count, expected) => {
    const onChange = vi.fn();
    await act(async () =>
      root.render(<CommandSearch value={value} tokens={tokens} onChange={onChange} />),
    );
    expect(input().placeholder).toBe('Search...');
    expect(host.querySelectorAll('button')).toHaveLength(0);
    await act(async () => input().focus());
    expect(host.querySelectorAll('button')).toHaveLength(count);
    const button = host.querySelector('button');
    if (!button) throw new Error('No suggestions');
    const down = new MouseEvent('mousedown', { bubbles: true, cancelable: true });
    button.dispatchEvent(down);
    expect(down.defaultPrevented).toBe(true);
    await act(async () => button.click());
    expect(onChange).toHaveBeenCalledExactlyOnceWith(expected);
  });

  it('hides unmatched tokens and applies custom field presentation', async () => {
    await act(async () =>
      root.render(
        <CommandSearch
          value="unknown"
          tokens={tokens}
          onChange={() => {}}
          placeholder="Search accounts"
          className="custom"
        />,
      ),
    );
    await act(async () => input().focus());
    expect(host.querySelectorAll('button')).toHaveLength(0);
    expect(input().placeholder).toBe('Search accounts');
    expect(host.firstElementChild?.className).toContain('custom');
  });

  it('propagates typed input and delays closing suggestions for clicks', async () => {
    const onChange = vi.fn();
    await act(async () =>
      root.render(<CommandSearch value="" tokens={tokens} onChange={onChange} />),
    );
    await act(async () => input().focus());
    await act(async () => {
      Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')?.set?.call(
        input(),
        'query',
      );
      input().dispatchEvent(new Event('input', { bubbles: true }));
    });
    expect(onChange).toHaveBeenCalledWith('query');
    await act(async () => input().blur());
    await act(async () => vi.advanceTimersByTime(99));
    expect(host.querySelectorAll('button')).toHaveLength(2);
    await act(async () => vi.advanceTimersByTime(1));
    expect(host.querySelectorAll('button')).toHaveLength(0);
  });
});
