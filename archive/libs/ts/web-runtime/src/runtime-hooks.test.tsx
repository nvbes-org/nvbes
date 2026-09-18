// @vitest-environment happy-dom
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, expect, it, vi } from 'vite-plus/test';
import { RelativeTime, formatRelativeDateTime, formatAbsoluteDateTime } from './relative-time';
import { LiveRegionProvider, useLiveRegion } from './live-region';
import { useNetworkQuality } from './use-network-quality';
import { getBrowserVisibilityState, useVisibilityAwareInterval } from './live-updates';

let root: Root;
let host: HTMLDivElement;
function VisibilityProbe({ hidden }: { hidden?: number | false }) {
  return <output>{String(useVisibilityAwareInterval(1000, hidden))}</output>;
}
it('updates polling cadence on visibility changes and removes its listener', async () => {
  const visibility = vi.spyOn(document, 'visibilityState', 'get').mockReturnValue('visible');
  const remove = vi.spyOn(document, 'removeEventListener');
  await act(async () => root.render(<VisibilityProbe />));
  expect(host.textContent).toBe('1000');
  visibility.mockReturnValue('hidden');
  await act(async () => document.dispatchEvent(new Event('visibilitychange')));
  expect(host.textContent).toBe('false');
  await act(async () => root.render(<VisibilityProbe hidden={10000} />));
  expect(host.textContent).toBe('10000');
  await act(async () => root.render(null));
  expect(remove).toHaveBeenCalledWith('visibilitychange', expect.any(Function));
});
it('defaults to visible without a browser document', () => {
  vi.stubGlobal('document', undefined);
  expect(getBrowserVisibilityState()).toBe('visible');
  vi.unstubAllGlobals();
  vi.stubGlobal('IS_REACT_ACT_ENVIRONMENT', true);
});
beforeEach(() => {
  vi.stubGlobal('IS_REACT_ACT_ENVIRONMENT', true);
  vi.useFakeTimers();
  vi.setSystemTime(new Date('2026-09-10T12:00:00Z'));
  host = document.createElement('div');
  document.body.append(host);
  root = createRoot(host);
});
afterEach(async () => {
  await act(async () => root.unmount());
  host.remove();
  vi.useRealTimers();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

it.each([
  [0, 'now'],
  [1, 'in 1 second'],
  [-1, '1 second ago'],
  [59, 'in 59 seconds'],
  [60, 'in 1 minute'],
  [3599, 'in 60 minutes'],
  [3600, 'in 1 hour'],
  [86399, 'in 24 hours'],
  [86400, 'tomorrow'],
  [-86400, 'yesterday'],
])('formats the relative boundary %s seconds', (seconds, expected) => {
  expect(formatRelativeDateTime(Date.now() + seconds * 1000)).toBe(expected);
});
it('preserves invalid input and accepts dates, strings and locale', () => {
  expect(formatRelativeDateTime('invalid')).toBe('invalid');
  expect(formatAbsoluteDateTime('invalid')).toBe('invalid');
  expect(formatRelativeDateTime(new Date('2026-09-10T12:00:00Z'), new Date(), 'fr')).toBe(
    'maintenant',
  );
  expect(formatAbsoluteDateTime('2026-09-10T12:00:00Z', 'en')).toContain('2026');
});
it('refreshes live timestamps and releases the interval on unmount', async () => {
  await act(async () =>
    root.render(<RelativeTime value="2026-09-10T12:00:00Z" className="timestamp" />),
  );
  const time = host.querySelector('time');
  expect(time?.dateTime).toBe('2026-09-10T12:00:00.000Z');
  expect(time?.className).toBe('timestamp');
  expect(time?.textContent).toBe('now');
  expect(time?.title).toContain('2026');
  await act(async () => vi.advanceTimersByTime(60_000));
  expect(time?.textContent).toBe('1 minute ago');
  await act(async () => root.render(null));
  expect(vi.getTimerCount()).toBe(0);
});
it('does not schedule fixed or disabled timestamps', async () => {
  await act(async () => root.render(<RelativeTime value="invalid" now={new Date()} />));
  expect(host.textContent).toBe('invalid');
  expect(host.querySelector('time')?.hasAttribute('datetime')).toBe(false);
  expect(vi.getTimerCount()).toBe(0);
  await act(async () => root.render(<RelativeTime value={Date.now()} refreshIntervalMs={0} />));
  expect(vi.getTimerCount()).toBe(0);
});
function Announcement() {
  const region = useLiveRegion();
  return (
    <button type="button" onClick={() => region.announce('Saved')}>
      Save
    </button>
  );
}
it('announces changes politely, including repeated messages', async () => {
  await act(async () =>
    root.render(
      <LiveRegionProvider>
        <Announcement />
      </LiveRegionProvider>,
    ),
  );
  const region = host.querySelector('[aria-live]');
  expect(region?.getAttribute('aria-live')).toBe('polite');
  expect(region?.getAttribute('aria-atomic')).toBe('true');
  expect(region?.className).toBe('sr-only');
  for (let i = 0; i < 2; i++) {
    await act(async () => host.querySelector('button')?.click());
    expect(region?.textContent).toBe('');
    await act(async () => vi.runOnlyPendingTimers());
    expect(region?.textContent).toBe('Saved');
  }
  await act(async () => root.render(<Announcement />));
  await act(async () => host.querySelector('button')?.click());
  expect(host.querySelector('[aria-live]')).toBeNull();
});
function NetworkProbe() {
  return <output>{JSON.stringify(useNetworkQuality())}</output>;
}
it('reports unknown network support without installing listeners', async () => {
  vi.stubGlobal('navigator', {});
  await act(async () => root.render(<NetworkProbe />));
  expect(JSON.parse(host.textContent ?? '')).toEqual({
    effectiveType: 'unknown',
    isSlowConnection: false,
    saveData: false,
  });
});
it('updates connection snapshots and unsubscribes from the same connection', async () => {
  const connection = Object.assign(new EventTarget(), {
    effectiveType: '4g',
    downlink: 10,
    rtt: 20,
    saveData: false,
  });
  const remove = vi.spyOn(connection, 'removeEventListener');
  vi.stubGlobal('navigator', { connection });
  await act(async () => root.render(<NetworkProbe />));
  for (const effectiveType of ['slow-2g', '2g', '3g', '4g', 'unknown']) {
    connection.effectiveType = effectiveType;
    await act(async () => connection.dispatchEvent(new Event('change')));
    expect(JSON.parse(host.textContent ?? '')).toEqual({
      effectiveType,
      isSlowConnection: ['slow-2g', '2g', '3g'].includes(effectiveType),
      downlink: 10,
      rtt: 20,
      saveData: false,
    });
  }
  for (const update of [{ downlink: 2 }, { rtt: 500 }, { saveData: true }]) {
    Object.assign(connection, update);
    await act(async () => connection.dispatchEvent(new Event('change')));
    expect(JSON.parse(host.textContent ?? '')).toMatchObject(update);
  }
  await act(async () => root.render(null));
  expect(remove).toHaveBeenCalledWith('change', expect.any(Function));
});
