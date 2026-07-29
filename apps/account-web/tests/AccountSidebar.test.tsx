import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it, vi } from 'vite-plus/test';

vi.mock('../src/components/AccountSidebar.shared', () => ({
  accountNavSections: [
    {
      title: 'Compte',
      items: [{ to: '/account', icon: () => null, label: 'Accueil' }],
    },
  ],
  SidebarNavItem: ({ label }: { label: string }) => <a href="/account">{label}</a>,
}));

vi.mock('../src/hooks/useAuthuser', () => ({
  useAuthuser: () => '0',
}));

import { AccountSidebar } from '../src/components/AccountSidebar';

describe('AccountSidebar', () => {
  it('uses native scrolling without injecting runtime styles', () => {
    const markup = renderToStaticMarkup(<AccountSidebar />);

    expect(markup).toContain('overflow-y-auto');
    expect(markup).not.toContain('<style');
  });
});
