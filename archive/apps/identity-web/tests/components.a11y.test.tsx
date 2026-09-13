// @vitest-environment happy-dom

import '@testing-library/jest-dom/vitest';

import { render } from '@testing-library/react';
import axe from 'axe-core';
import { describe, expect, it } from 'vite-plus/test';

import { Button } from '../src/components/ui/button';
import { Field, FieldLabel } from '../src/components/ui/field';
import { Input } from '../src/components/ui/input';
import { Switch } from '../src/components/ui/switch';

const wcagTags = ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'wcag22aa'];

async function checkA11y(container: HTMLElement) {
  const results = await axe.run(container, {
    runOnly: {
      type: 'tag',
      values: wcagTags,
    },
  });

  return results.violations;
}

describe('@a11y Component Primitives Accessibility', () => {
  it('Button renders accessible with sufficient name and no violations', async () => {
    const { container } = render(
      <main>
        <Button type="button">Continuer vers mon compte</Button>
      </main>,
    );

    const violations = await checkA11y(container);
    expect(violations).toEqual([]);
  });

  it('Field with FieldLabel and Input associates labels correctly', async () => {
    const { container } = render(
      <main>
        <Field>
          <FieldLabel htmlFor="test-email-field">Adresse Email</FieldLabel>
          <Input id="test-email-field" type="email" placeholder="nom@exemple.fr" />
        </Field>
      </main>,
    );

    const violations = await checkA11y(container);
    expect(violations).toEqual([]);
  });

  it('Switch primitive exposes accessible role and state without violations', async () => {
    const { container } = render(
      <main>
        <label htmlFor="test-switch">Recevoir les notifications de sécurité</label>
        <Switch id="test-switch" aria-label="Recevoir les notifications de sécurité" />
      </main>,
    );

    const violations = await checkA11y(container);
    expect(violations).toEqual([]);
  });
});
