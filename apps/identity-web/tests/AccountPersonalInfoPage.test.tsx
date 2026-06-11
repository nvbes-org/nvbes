import type { AccountPrincipal } from '@nvbes/identity-client';
import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vite-plus/test';

import { PersonalInfoCard } from '../src/pages/AccountPersonalInfoPage.form';

function user(): AccountPrincipal {
  return {
    id: 'user-1',
    email: 'rayane@example.test',
    display_name: 'Rayane Guemmoud',
    firstname: 'Rayane',
    lastname: 'Guemmoud',
    username: 'rayane',
    birthdate: '2000-01-01',
    region: 'FR',
    email_verified: true,
    mfa_enabled: false,
    created_at: '2026-06-09T10:00:00Z',
  };
}

describe('PersonalInfoCard', () => {
  it('renders account summary and editable fields inside one card', () => {
    const markup = renderToStaticMarkup(
      <PersonalInfoCard
        user={user()}
        fullName="Rayane Guemmoud"
        memberSince="9 juin 2026"
        firstname="Rayane"
        lastname="Guemmoud"
        username="rayane"
        birthdate="2000-01-01"
        region="FR"
        editError={null}
        editSuccess={false}
        loading={false}
        onFirstnameChange={() => undefined}
        onLastnameChange={() => undefined}
        onUsernameChange={() => undefined}
        onBirthdateChange={() => undefined}
        onRegionChange={() => undefined}
        onSubmit={() => undefined}
      />,
    );

    expect(markup.match(/data-slot="card"/g)).toHaveLength(1);
    expect(markup).toContain('Profil');
    expect(markup).toContain('Membre depuis');
    expect(markup).toContain('data-slot="editable"');
    expect(markup).toContain('Rayane');
  });
});
