// @vitest-environment happy-dom

import '@testing-library/jest-dom/vitest';

import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { KeyRound } from 'lucide-react';
import { describe, expect, it, vi } from 'vite-plus/test';
import { IdentityPageHeader } from '../src/components/IdentityPage';
import { MfaMethodChoiceList } from '../src/components/MfaMethodChoiceList';
import { SecuritySetupForm } from '../src/components/SecuritySetupForm';

describe('shared Account components', () => {
  it('keeps page headers semantic and delegates back navigation', async () => {
    const user = userEvent.setup();
    const onBack = vi.fn();

    render(<IdentityPageHeader title="Sécurité" description="Gérez vos accès." onBack={onBack} />);

    expect(screen.getByRole('heading', { level: 1, name: 'Sécurité' })).toBeVisible();
    expect(screen.getByText('Gérez vos accès.')).toBeVisible();

    await user.click(screen.getByRole('button', { name: 'Retour' }));
    expect(onBack).toHaveBeenCalledOnce();
  });

  it('renders MFA choices from data and reports the selected method', async () => {
    const user = userEvent.setup();
    const onSelect = vi.fn();

    render(
      <MfaMethodChoiceList
        choices={[
          {
            icon: KeyRound,
            label: 'Clé de sécurité',
            value: 'webauthn',
          },
        ]}
        onSelect={onSelect}
      />,
    );

    await user.click(screen.getByRole('button', { name: 'Clé de sécurité' }));
    expect(onSelect).toHaveBeenCalledExactlyOnceWith('webauthn');
  });

  it('centralizes setup form feedback and disabled submit state', async () => {
    const user = userEvent.setup();
    const onValueChange = vi.fn();
    const onCancel = vi.fn();

    render(
      <SecuritySetupForm
        title="Configurer TOTP"
        fieldId="factor-label"
        fieldLabel="Nom"
        fieldPlaceholder="Téléphone"
        value=""
        error="Configuration impossible"
        loading={false}
        submitLabel="Continuer"
        pendingLabel="Chargement"
        submitDisabled
        onValueChange={onValueChange}
        onCancel={onCancel}
        onSubmit={() => {}}
      />,
    );

    await user.type(screen.getByLabelText('Nom'), 'Mobile');
    expect(onValueChange).toHaveBeenCalled();
    expect(screen.getByRole('alert')).toHaveTextContent('Configuration impossible');
    expect(screen.getByRole('button', { name: 'Continuer' })).toBeDisabled();

    await user.click(screen.getByRole('button', { name: 'Annuler' }));
    expect(onCancel).toHaveBeenCalledOnce();
  });
});
