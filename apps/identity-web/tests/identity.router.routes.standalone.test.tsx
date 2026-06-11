import { createRootRoute } from '@tanstack/react-router';
import { describe, expect, it } from 'vite-plus/test';
import { createStandaloneRoutes } from '../src/identity.router.routes.standalone';

describe('createStandaloneRoutes', () => {
  it('redirects the root path to login instead of rendering a landing page', () => {
    const root = createRootRoute();
    const [rootRoute] = createStandaloneRoutes(root);

    expect(rootRoute).toBeDefined();
    expect(rootRoute?.options.component).toBeUndefined();
    expect(rootRoute?.options.beforeLoad).toBeDefined();
  });
});
