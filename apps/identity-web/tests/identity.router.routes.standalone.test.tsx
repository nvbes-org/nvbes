import { createRootRoute } from '@tanstack/react-router';
import { describe, expect, it } from 'vite-plus/test';
import { LazyIdentitySecurityPage } from '../src/identity.router.pages';
import { router } from '../src/identity.router';
import { createStandaloneRoutes } from '../src/identity.router.routes.standalone';

describe('createStandaloneRoutes', () => {
  it('redirects the root path to login instead of rendering a landing page', () => {
    const root = createRootRoute();
    const [rootRoute] = createStandaloneRoutes(root);

    expect(rootRoute).toBeDefined();
    expect(rootRoute?.options.component).toBeUndefined();
    expect(rootRoute?.options.beforeLoad).toBeDefined();
  });

  it('preloads lazy route modules when navigation intent is detected', () => {
    expect(router.options.defaultPreload).toBe('intent');
    expect(typeof LazyIdentitySecurityPage.preload).toBe('function');
  });
});
