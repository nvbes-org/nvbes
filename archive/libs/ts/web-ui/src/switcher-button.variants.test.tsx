import { renderToStaticMarkup } from 'react-dom/server';
import { expect, it } from 'vite-plus/test';
import { SwitcherButton } from './multi-account-switcher.parts';

it('preserves the outline small button contract and default non-submit type', () => {
  expect(
    renderToStaticMarkup(
      <SwitcherButton variant="outline" size="sm">
        Cancel
      </SwitcherButton>,
    ),
  ).toBe(
    '<button type="button" class="inline-flex shrink-0 items-center justify-center gap-1.5 rounded-lg font-medium transition outline-none disabled:pointer-events-none disabled:opacity-50 border border-border bg-background text-foreground hover:bg-muted h-7 px-2.5 text-[0.8rem]">Cancel</button>',
  );
});
