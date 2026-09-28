import { describe, expect, it } from 'vitest';
import { normalizeGeneratedHtml } from './email-ui.normalize';

describe('normalizeGeneratedHtml', () => {
  it('trims surrounding whitespace and ends with a single newline', () => {
    expect(normalizeGeneratedHtml('  <html />\n\n')).toBe('<html />\n');
    expect(normalizeGeneratedHtml('<html />')).toBe('<html />\n');
  });
});
