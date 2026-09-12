// @vitest-environment happy-dom
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vite-plus/test';
import { MultiAccountSwitcher, type MultiAccountSwitcherProps } from './multi-account-switcher';

let root: Root;
let host: HTMLDivElement;
beforeEach(() => {
  vi.stubGlobal('IS_REACT_ACT_ENVIRONMENT', true);
  host = document.createElement('div');
  document.body.append(host);
  root = createRoot(host);
});
afterEach(async () => {
  await act(async () => root.unmount());
  host.remove();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});
function trigger() {
  const button = host.querySelector('button');
  if (!button) throw new Error('Missing switcher trigger');
  return button;
}
function portal() {
  return document.querySelector<HTMLElement>('[data-switcher-portal]');
}
function props(overrides: Partial<MultiAccountSwitcherProps> = {}): MultiAccountSwitcherProps {
  return {
    accounts: [
      { id: 'ada', displayName: 'Ada Lovelace', email: 'ada@example.test', isActive: true },
      { id: 'grace', displayName: 'Grace Hopper', email: 'grace@example.test', isActive: false },
    ],
    loading: false,
    onSelectAccount: vi.fn(),
    onAddAccount: vi.fn(),
    ...overrides,
  };
}

describe('account switcher interactions', () => {
  it.each(['default', 'compact'] as const)(
    'selects, removes and adds accounts in %s density',
    async (density) => {
      const options = props({
        density,
        onRemoveAccount: vi.fn(),
        footerActions: <span>Session help</span>,
        hideCurrentAccountDetailsOnMobile: true,
      });
      await act(async () => root.render(<MultiAccountSwitcher {...options} />));
      expect(portal()).toBeNull();
      await act(async () => trigger().click());
      expect(portal()?.textContent).toContain('Comptes connectés');
      expect(portal()?.textContent).toContain('Basculer entre les sessions ouvertes.');
      expect(portal()?.textContent).toContain('Session help');
      expect(portal()?.style.width).toBe(density === 'compact' ? '320px' : '384px');
      expect(host.innerHTML).toMatchSnapshot(`${density} active account control`);
      expect(portal()?.outerHTML).toMatchSnapshot(`${density} connected accounts menu`);
      const buttons = [...(portal()?.querySelectorAll('button') ?? [])];
      const select = buttons.find((button) => button.textContent?.includes('Grace Hopper'));
      const remove = buttons.find(
        (button) => button.getAttribute('aria-label') === 'Retirer Grace Hopper',
      );
      const add = buttons.find((button) =>
        button.textContent?.includes('Se connecter à un autre compte'),
      );
      if (!select || !remove || !add) throw new Error('Missing account actions');
      await act(async () => {
        select.click();
        remove.click();
        add.click();
      });
      expect(options.onSelectAccount).toHaveBeenCalledExactlyOnceWith('grace');
      expect(options.onRemoveAccount).toHaveBeenCalledExactlyOnceWith('grace');
      expect(options.onAddAccount).toHaveBeenCalledTimes(1);
      expect(
        buttons.some((button) => button.getAttribute('aria-label') === 'Retirer Ada Lovelace'),
      ).toBe(false);
      await act(async () => trigger().click());
      expect(portal()).toBeNull();
    },
  );

  it('keeps inside clicks open, closes outside clicks and removes listeners', async () => {
    const add = vi.spyOn(window, 'addEventListener');
    const remove = vi.spyOn(window, 'removeEventListener');
    const documentAdd = vi.spyOn(document, 'addEventListener');
    const documentRemove = vi.spyOn(document, 'removeEventListener');
    await act(async () => root.render(<MultiAccountSwitcher {...props()} />));
    expect(add.mock.calls.filter(([name]) => ['scroll', 'resize'].includes(name))).toEqual([]);
    expect(documentAdd.mock.calls.filter(([name]) => name === 'pointerdown')).toEqual([]);
    await act(async () => trigger().click());
    const menu = portal();
    if (!menu) throw new Error('Missing portal');
    await act(async () => trigger().dispatchEvent(new Event('pointerdown', { bubbles: true })));
    expect(portal()).not.toBeNull();
    const text = document.createTextNode('Text target');
    menu.append(text);
    await act(async () => text.dispatchEvent(new Event('pointerdown', { bubbles: true })));
    expect(portal()).not.toBeNull();
    const anchor = host.firstElementChild;
    if (!anchor) throw new Error('Missing anchor');
    vi.spyOn(anchor, 'getBoundingClientRect').mockReturnValue(new DOMRect(10, 20, 400, 40));
    await act(async () => window.dispatchEvent(new Event('resize')));
    expect(menu.style.top).toBe('64px');
    expect(menu.style.left).toBe('10px');
    expect(menu.style.width).toBe('400px');
    await act(async () => document.dispatchEvent(new Event('pointerdown')));
    expect(portal()).toBeNull();
    for (const kind of ['scroll', 'resize']) {
      const registration = add.mock.calls.find(([name]) => name === kind);
      expect(registration).toBeDefined();
      expect(
        remove.mock.calls.some(([name, handler]) => name === kind && handler === registration?.[1]),
      ).toBe(true);
    }
    expect(add).toHaveBeenCalledWith('scroll', expect.any(Function), true);
    expect(remove).toHaveBeenCalledWith('scroll', expect.any(Function), true);
    const registration = documentAdd.mock.calls.find(([name]) => name === 'pointerdown');
    expect(documentRemove).toHaveBeenCalledWith('pointerdown', registration?.[1]);
  });

  it.each([true, false])('renders loading=%s and empty account states', async (loading) => {
    await act(async () =>
      root.render(<MultiAccountSwitcher {...props({ accounts: [], loading })} />),
    );
    expect(trigger().textContent).toContain('Aucun compte actif');
    await act(async () => trigger().click());
    expect(portal()?.textContent).toContain(
      loading ? 'Chargement des comptes...' : 'Aucun compte connecté.',
    );
    expect(portal()?.querySelector('[role="status"]') !== null).toBe(loading);
    expect(portal()?.outerHTML).toMatchSnapshot(`accounts loading=${loading}`);
  });

  it('disables the switching account and uses custom labels', async () => {
    const options = props({
      switchingAccountId: 'grace',
      menuTitle: 'Accounts',
      menuDescription: 'Pick one',
      addAccountLabel: 'Add account',
    });
    await act(async () => root.render(<MultiAccountSwitcher {...options} />));
    await act(async () => trigger().click());
    const disabled = portal()?.querySelector<HTMLButtonElement>('button:disabled');
    expect(disabled?.textContent).toContain('Grace Hopper');
    expect(disabled?.querySelector('[role="status"]')).not.toBeNull();
    await act(async () => disabled?.click());
    expect(options.onSelectAccount).not.toHaveBeenCalled();
    expect(portal()?.textContent).toContain('Accounts');
    expect(portal()?.textContent).toContain('Pick one');
    expect(portal()?.textContent).toContain('Add account');
    expect(portal()?.querySelector('[aria-label^="Retirer"]')).toBeNull();
    expect(host.innerHTML).toMatchSnapshot('desktop account details');
    expect(portal()?.outerHTML).toMatchSnapshot('switching account menu');
  });

  it.each(['default', 'compact'] as const)(
    'keeps avatar fallbacks after image failures (%s)',
    async (density) => {
      const options = props({ density });
      options.accounts[0].avatarUrl = 'https://cdn.example.test/ada.png';
      options.accounts[0].avatarFallback = 'AD';
      options.accounts[1].avatarUrl = 'https://cdn.example.test/grace.png';
      await act(async () => root.render(<MultiAccountSwitcher {...options} />));
      await act(async () => trigger().click());
      const images = [...document.querySelectorAll('img')];
      expect(images).toHaveLength(3);
      expect(host.innerHTML).toMatchSnapshot(`${density} avatar control`);
      expect(portal()?.outerHTML).toMatchSnapshot(`${density} avatar menu`);
      await act(async () => images.forEach((image) => image.dispatchEvent(new Event('error'))));
      for (const image of images) {
        expect(image.crossOrigin).toBe('anonymous');
        expect(image.style.display).toBe('none');
      }
      expect(trigger().textContent).toContain('AD');
      expect(portal()?.textContent).toContain('GH');
    },
  );

  it('renders unnamed sessions using the generic account icon', async () => {
    const options = props({
      accounts: [{ id: 'anonymous', displayName: '', email: '', isActive: false }],
    });
    await act(async () => root.render(<MultiAccountSwitcher {...options} />));
    await act(async () => trigger().click());
    expect(portal()?.querySelectorAll('svg.lucide-user-round').length).toBe(1);
  });
});
