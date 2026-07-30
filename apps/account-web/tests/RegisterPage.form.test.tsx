// @vitest-environment happy-dom

import '@testing-library/jest-dom/vitest';

import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vite-plus/test';
import { RegisterForm } from '../src/pages/RegisterPage.form';

describe('RegisterForm', () => {
  const defaultProps = {
    canSubmit: false,
    checkingAuth: false,
    email: '',
    emailAlreadyExists: false,
    emailAvailability: 'idle' as const,
    emailValid: false,
    error: null,
    handleEmailChange: () => {},
    handleSubmit: async () => {},
    handleUsernameChange: () => {},
    legalDocumentsAccepted: false,
    loading: false,
    marketingEmailsAccepted: false,
    password: '',
    passwordValid: false,
    setLegalDocumentsAccepted: () => {},
    setMarketingEmailsAccepted: () => {},
    setPassword: () => {},
    username: '',
    usernameTaken: false,
    usernameAvailability: 'idle' as const,
    usernameValid: false,
  };

  it('collects minimal registration data with email as the credential identifier', () => {
    const markup = renderToStaticMarkup(<RegisterForm {...defaultProps} />);

    expect(markup.match(/<form/gu)).toHaveLength(1);
    expect(markup).toContain('autoComplete="on"');
    expect(markup).toContain('name="email"');
    expect(markup).toContain('autoComplete="username"');
    expect(markup).toContain('name="nickname"');
    expect(markup).toContain('autoComplete="nickname"');
    expect(markup).toContain('name="password"');
    expect(markup).toContain('autoComplete="new-password"');
    expect(markup).toContain('maxLength="100"');
    expect(markup).toContain('minLength="8"');
    expect(markup).not.toContain('caractères max.');
    expect(markup).toContain('Utilisez au moins 8 caractères.');
    expect(markup).not.toContain('register-firstname');
    expect(markup).not.toContain('register-lastname');
    expect(markup).not.toContain('register-birthdate');
    expect(markup).not.toContain('register-region');
  });

  it('shows validation after the field was visited and updates it while typing', async () => {
    const user = userEvent.setup();
    const { rerender } = render(<RegisterForm {...defaultProps} email="ss" />);
    const emailInput = screen.getByLabelText(/^Email/u);

    expect(screen.queryByText('Saisissez une adresse email valide.')).not.toBeInTheDocument();

    await user.click(emailInput);
    await user.tab();

    expect(emailInput).toHaveAttribute('aria-invalid', 'true');
    expect(screen.getByText('Saisissez une adresse email valide.')).toBeVisible();

    rerender(
      <RegisterForm
        {...defaultProps}
        email="utilisateur@nvbes.fr"
        emailAvailability="unavailable"
        emailValid
      />,
    );

    expect(emailInput).toHaveAttribute('aria-invalid', 'true');
    expect(screen.getByText('Cet email est déjà utilisé.')).toBeVisible();

    rerender(
      <RegisterForm
        {...defaultProps}
        email="utilisateur@nvbes.fr"
        emailAvailability="available"
        emailValid
      />,
    );

    expect(emailInput).toHaveAttribute('aria-invalid', 'false');
    expect(screen.getByText('Adresse email valide.')).toBeVisible();
  });
});
