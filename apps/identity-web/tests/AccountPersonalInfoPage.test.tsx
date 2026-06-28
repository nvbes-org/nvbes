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
  it('renders account form content without card chrome or duplicated identity summaries', () => {
    const markup = renderToStaticMarkup(
      <PersonalInfoCard
        user={user()}
        memberSince="9 juin 2026"
        firstname="Rayane"
        lastname="Guemmoud"
        username="rayane"
        birthdate="2000-01-01"
        region="FR"
        regionLoading={false}
        regions={[
          {
            country_code: 'FR',
            data_region: 'eu-west',
            legal_jurisdiction: 'FR',
            primary_timezone: 'Europe/Paris',
            timezones: ['Europe/Paris'],
            sub_region: 'Europe',
            display_name: 'France',
          },
        ]}
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

    expect(markup).not.toContain('data-slot="card"');
    expect(markup).toContain('Compte cree le 9 juin 2026');
    expect(markup).toContain('Email verifie');
    expect(markup).toContain('id="account-firstname"');
    expect(markup).toContain('id="account-birthdate"');
    expect(markup).toContain('id="account-region"');
    expect(markup).not.toContain('Nom complet');
    expect(markup).not.toContain('rayane@example.test');
  });
});
