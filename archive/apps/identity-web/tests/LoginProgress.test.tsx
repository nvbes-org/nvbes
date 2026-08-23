import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vite-plus/test';

import { LoginProgress } from '../src/pages/LoginProgress';

describe('LoginProgress', () => {
  it('shows two steps outside OAuth login flows', () => {
    const markup = renderToStaticMarkup(<LoginProgress isOAuthFlow={false} step="identifier" />);

    expect(markup).toContain('Connexion en 2 étapes');
  });

  it('shows three steps during OAuth login flows', () => {
    const markup = renderToStaticMarkup(<LoginProgress isOAuthFlow={true} step="identifier" />);

    expect(markup).toContain('Connexion en 3 étapes');
  });
});
