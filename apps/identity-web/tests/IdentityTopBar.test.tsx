import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it, vi } from 'vite-plus/test';

vi.mock('@tanstack/react-router', () => ({
  Link: ({ children }: { children: React.ReactNode }) => <a href="/login">{children}</a>,
  useRouterState: () => '/login',
}));

vi.mock('@/components/IdentitySidebar', () => ({
  IdentitySidebar: () => <nav>Navigation Identity</nav>,
}));

import { IdentityTopBar } from '../src/components/IdentityTopBar';

describe('IdentityTopBar', () => {
  it('renders the mobile navigation trigger inside its sheet root', () => {
    expect(() => renderToStaticMarkup(<IdentityTopBar />)).not.toThrow();
  });
});
