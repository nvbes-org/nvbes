// @vitest-environment happy-dom
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, expect, it, vi } from 'vite-plus/test';
import { SharedTrackingConsentBanner } from './TrackingConsentBanner';
import {
  ACCEPT_ALL_CONSENT,
  DECLINE_ALL_CONSENT,
  type CookieConsentState,
} from './tracking-consent';

let root: Root;
let host: HTMLDivElement;
const reload = vi.fn();
beforeEach(() => {
  vi.stubGlobal('IS_REACT_ACT_ENVIRONMENT', true);
  host = document.createElement('div');
  document.body.append(host);
  root = createRoot(host);
  reload.mockClear();
  vi.spyOn(window.location, 'reload').mockImplementation(reload);
});
afterEach(async () => {
  await act(async () => root.unmount());
  host.remove();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});
async function click(label: string) {
  const button = [...host.querySelectorAll('button')].find(
    (item) => item.textContent === label || item.getAttribute('aria-label') === label,
  );
  if (!button) throw new Error(`Missing button: ${label}`);
  await act(async () => button.click());
}
async function renderBanner(consent: CookieConsentState | null = null) {
  const save = vi.fn();
  await act(async () =>
    root.render(
      <SharedTrackingConsentBanner
        getTrackingConsent={() => consent}
        setTrackingConsent={save}
        sourcePrefix="test"
      />,
    ),
  );
  return save;
}

it('does not interrupt users who already chose', async () => {
  const save = await renderBanner(DECLINE_ALL_CONSENT);
  expect(host.textContent).toBe('');
  expect(save).not.toHaveBeenCalled();
});
it.each([
  ['Tout accepter', ACCEPT_ALL_CONSENT, 'accept-all'],
  ['Tout refuser', DECLINE_ALL_CONSENT, 'decline-all'],
] as const)('persists %s before reloading', async (label, expected, source) => {
  const save = await renderBanner();
  expect(host.querySelector('a')?.getAttribute('href')).toBe('/legal/privacy-policy');
  await click(label);
  expect(save).toHaveBeenCalledExactlyOnceWith(expected, `test:banner:${source}`);
  expect(reload).toHaveBeenCalledTimes(1);
  expect(save.mock.invocationCallOrder[0]).toBeLessThan(reload.mock.invocationCallOrder[0]);
});

it('saves explicitly chosen purposes and keeps the essential category enabled', async () => {
  const save = await renderBanner();
  await click('Personnaliser');
  expect(host.querySelectorAll('[role="switch"]')).toHaveLength(4);
  await click("Analyse d'audience");
  await click("Mesure d'audience produit");
  await click('Performance & erreurs');
  await click("Rapports d'erreurs navigateur");
  for (const toggle of host.querySelectorAll('[role="switch"]'))
    expect(toggle.getAttribute('aria-checked')).toBe('false');
  expect(save).not.toHaveBeenCalled();
  await click("Mesure d'audience produit");
  await click('Enregistrer mes choix');
  expect(save).toHaveBeenCalledWith(
    expect.objectContaining({
      categories: { essentials: true, analytics: true, performance: false },
      analytics: { ...DECLINE_ALL_CONSENT.analytics, productAnalytics: true },
    }),
    'test:banner:custom',
  );
  expect(reload).toHaveBeenCalledTimes(1);
});

it('allows returning from customization without persisting draft changes', async () => {
  const save = await renderBanner();
  await click('Personnaliser');
  await click("Analyse d'audience");
  await click('Retour');
  expect(host.querySelector('[role="switch"]')).toBeNull();
  expect(save).not.toHaveBeenCalled();
  expect(reload).not.toHaveBeenCalled();
  await click('Tout refuser');
  expect(save).toHaveBeenCalledWith(DECLINE_ALL_CONSENT, 'test:banner:decline-all');
});
