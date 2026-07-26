import { describe, expect, it } from 'vite-plus/test';
import {
  safeHtmlToString,
  safeStyleElementCssToString,
  safeUrlToString,
  sanitizeHtml,
  sanitizeStyleElementCss,
  sanitizeUrlForAttribute,
} from './index';

describe('XSS prevention primitives', () => {
  it('sanitizes executable HTML before branding safe HTML', () => {
    const html = sanitizeHtml(
      '<p onclick="steal()" style="background:url(javascript:alert(1))">Hello</p><!--x--><script>alert(1)</script><iframe srcdoc="<script>alert(1)</script>"></iframe>',
    );

    expect(safeHtmlToString(html)).toBe('<p>Hello</p>');
  });

  it('rejects scriptable URLs for DOM attributes', () => {
    expect(sanitizeUrlForAttribute('javascript:alert(1)')).toBeUndefined();
    expect(sanitizeUrlForAttribute('java\u0000script:alert(1)')).toBeUndefined();
    expect(sanitizeUrlForAttribute('data:text/html,<script>alert(1)</script>')).toBeUndefined();
    expect(sanitizeUrlForAttribute('/account/settings')).toBe('/account/settings');
    expect(safeUrlToString(sanitizeUrlForAttribute('https://nvbes.fr')!)).toBe('https://nvbes.fr');
  });

  it('rejects unsafe CSS for style elements', () => {
    expect(safeStyleElementCssToString(sanitizeStyleElementCss('button { color: red; }'))).toBe(
      'button { color: red; }',
    );
    expect(safeStyleElementCssToString(sanitizeStyleElementCss('@import url(x.css);'))).toBe('');
    expect(
      safeStyleElementCssToString(sanitizeStyleElementCss('</style><script>x()</script>')),
    ).toBe('');
  });
});
