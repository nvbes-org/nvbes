import { render } from 'react-email';
import { describe, expect, it } from 'vitest';
import { EmailActionTemplate } from './email-ui.action-template';

describe('EmailActionTemplate', () => {
  it('renders the Account brand and an accessible action link', async () => {
    const html = await render(
      <EmailActionTemplate
        actionLabel="Verify email"
        actionUrl="https://identity.nvbes.fr/verify"
        expiry="August 4, 2026 at 12:37"
        greeting="Hi Ada,"
        message="Verify your email address to activate your nvbes account."
        preview="Verify your email address"
        timing="This link expires on"
        timezone="Europe/Paris (UTC+02:00)"
        title="Verify your email address"
        year="2026"
      />,
    );

    expect(html).toContain('nvbes <span');
    expect(html).toContain('Account</span>');
    expect(html).toContain('href="https://identity.nvbes.fr/verify"');
    expect(html).toContain('August 4, 2026 at 12:37');
    expect(html).toContain('Europe/Paris (UTC+02:00)');
    expect(html).toContain('Privacy policy');
    expect(html).toContain('Data processing agreement');
  });

  it('uses Outlook-safe layout fallbacks for the action content', async () => {
    const html = await render(
      <EmailActionTemplate
        actionLabel="Verify email"
        actionUrl="https://identity.nvbes.fr/verify"
        expiry="August 4, 2026 at 12:37"
        greeting="Hi Ada,"
        message="Verify your email address to activate your nvbes account."
        preview="Verify your email address"
        timing="This link expires on"
        timezone="Europe/Paris (UTC+02:00)"
        title="Verify your email address"
        year="2026"
      />,
    );

    expect(html).toContain('mso-line-height-rule:exactly');
    expect(html).toContain('bgcolor="#247f7b"');
    expect(html).not.toContain('margin:0 auto');
    expect(html).not.toContain('min-width:180px');
    expect(html).not.toContain('font-weight:550');
    expect(html).not.toContain('font-weight:650');
  });
});
