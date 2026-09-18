// @vitest-environment happy-dom
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, expect, it, vi } from 'vite-plus/test';
import { VersionMismatchBanner } from './version-mismatch-banner';
import { verifiedFetchJson } from './verified-fetch';
vi.mock('./verified-fetch', () => ({ verifiedFetchJson: vi.fn() }));

let root: Root;
let host: HTMLDivElement;
beforeEach(() => {
  vi.stubGlobal('IS_REACT_ACT_ENVIRONMENT', true);
  const storage = new Map<string, string>();
  vi.stubGlobal('localStorage', {
    getItem: (key: string) => storage.get(key) ?? null,
    setItem: (key: string, value: string) => storage.set(key, value),
    removeItem: (key: string) => storage.delete(key),
  });
  host = document.createElement('div');
  document.body.append(host);
  root = createRoot(host);
});
afterEach(async () => {
  await act(async () => root.unmount());
  host.remove();
  vi.restoreAllMocks();
  vi.clearAllMocks();
  vi.unstubAllGlobals();
});
const banner = (build = 'front') => (
  <VersionMismatchBanner
    healthUrl="https://api.test/health"
    frontendBuildId={build}
    appName="test"
  />
);

it('warns on a release mismatch and offers an explicit reload', async () => {
  vi.mocked(verifiedFetchJson).mockResolvedValue({ release_id: 'back' });
  const reload = vi.spyOn(window.location, 'reload').mockImplementation(() => {});
  await act(async () => root.render(banner()));
  expect(host.querySelector('h2')?.textContent).toBe('Nouvelle version disponible');
  expect(verifiedFetchJson).toHaveBeenCalledWith('https://api.test/health', expect.anything(), {
    allowedOrigins: ['https://api.test/health'],
    cache: 'no-store',
  });
  await act(async () => host.querySelector('button')?.click());
  expect(reload).toHaveBeenCalledTimes(1);
});

it('rechecks a settled URL after remount instead of caching a health response forever', async () => {
  vi.mocked(verifiedFetchJson)
    .mockResolvedValueOnce({ release_id: 'front' })
    .mockResolvedValueOnce({ release_id: 'back' });
  await act(async () => root.render(banner()));
  expect(host.textContent).toBe('');
  await act(async () => root.render(null));
  await act(async () => root.render(banner()));
  expect(host.textContent).toContain('Nouvelle version');
  expect(verifiedFetchJson).toHaveBeenCalledTimes(2);
});

it('does not poison subsequent checks after a failed health request', async () => {
  vi.mocked(verifiedFetchJson)
    .mockRejectedValueOnce(new Error('offline'))
    .mockResolvedValueOnce({ release_id: 'back' });
  await act(async () => root.render(banner()));
  expect(host.textContent).toBe('');
  await act(async () => root.render(null));
  await act(async () => root.render(banner()));
  expect(host.textContent).toContain('Nouvelle version');
});

it('clears an obsolete warning when the build matches again', async () => {
  vi.mocked(verifiedFetchJson).mockResolvedValue({ release_id: 'back' });
  await act(async () => root.render(banner()));
  expect(host.querySelector('aside')).not.toBeNull();
  await act(async () => root.render(banner('back')));
  expect(host.textContent).toBe('');
});

it('deduplicates in-flight requests and ignores results after unmount', async () => {
  let resolve: (value: { release_id: string }) => void = () => {};
  vi.mocked(verifiedFetchJson).mockReturnValue(
    new Promise((done) => {
      resolve = done;
    }),
  );
  await act(async () =>
    root.render(
      <>
        {banner()}
        {banner()}
      </>,
    ),
  );
  expect(verifiedFetchJson).toHaveBeenCalledTimes(1);
  await act(async () => root.render(null));
  await act(async () => resolve({ release_id: 'back' }));
  expect(host.textContent).toBe('');
  expect(localStorage.getItem('nvbes.version-mismatch.test.lastPromptAt')).toBeNull();
});
