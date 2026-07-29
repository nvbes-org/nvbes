// @vitest-environment happy-dom

import '@testing-library/jest-dom/vitest';

import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vite-plus/test';

import { TrackingConsentToggle } from '../src/components/TrackingConsentToggle';

describe('TrackingConsentToggle', () => {
  it('exposes an accessible switch and reports a keyboard-equivalent toggle', async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();

    render(
      <TrackingConsentToggle
        checked={false}
        onChange={onChange}
        label="Mesure d'audience"
        description="Mesure anonyme de la fréquentation."
      />,
    );

    const toggle = screen.getByRole('switch', { name: "Mesure d'audience" });
    expect(toggle).not.toBeChecked();
    expect(screen.getByText('Mesure anonyme de la fréquentation.')).toBeVisible();

    await user.tab();
    expect(toggle).toHaveFocus();
    await user.keyboard(' ');

    expect(onChange).toHaveBeenCalledExactlyOnceWith(true);
  });
});
