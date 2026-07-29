import type { AccountPrincipal } from '@nvbes/identity-client';
import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it, vi } from 'vite-plus/test';

vi.mock('../src/pages/RegisterPage.region', () => ({
  RegionSelect: ({ id }: { id: string }) => <div id={id} />,
}));

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
  it('renders profile form inside a single parent card', () => {
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
        isPersonalInfoInvalid={false}
        isPersonalInfoUnchanged={true}
        errors={{}}
        onFirstnameChange={() => undefined}
        onLastnameChange={() => undefined}
        onUsernameChange={() => undefined}
        onBirthdateChange={() => undefined}
        onRegionChange={() => undefined}
        onSubmit={() => undefined}
      />,
    );

    expect(markup.match(/data-slot="card"/g)).toHaveLength(1);
    expect(markup).toContain('id="account-firstname"');
    expect(markup).toContain('id="account-birthdate"');
    expect(markup).toContain('id="account-region"');
    expect(markup).toContain('required=""');
    expect(markup).not.toContain('Nom complet');
    expect(markup).not.toContain('rayane@example.test');
  });

  it('disables saving and exposes field errors when personal information is invalid', () => {
    const markup = renderToStaticMarkup(
      <PersonalInfoCard
        user={user()}
        memberSince="9 juin 2026"
        firstname="Rayane"
        lastname="Guemmoud"
        username=""
        birthdate="2000-01-01"
        region="FR"
        regionLoading={false}
        regions={[]}
        editError={null}
        editSuccess={false}
        loading={false}
        isPersonalInfoInvalid
        isPersonalInfoUnchanged={false}
        errors={{ username: 'Le nom d’utilisateur est requis.' }}
        onFirstnameChange={() => undefined}
        onLastnameChange={() => undefined}
        onUsernameChange={() => undefined}
        onBirthdateChange={() => undefined}
        onRegionChange={() => undefined}
        onSubmit={() => undefined}
      />,
    );

    expect(markup).toContain('aria-invalid="true"');
    expect(markup).toContain('Le nom d’utilisateur est requis.');
    expect(markup).toMatch(/<button[^>]*type="submit"[^>]*disabled=""/);
  });
});
