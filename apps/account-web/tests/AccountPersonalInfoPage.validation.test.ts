import { describe, expect, it } from 'vite-plus/test';
import { validatePersonalInfo } from '../src/pages/AccountPersonalInfoPage.validation';

const validInput = {
  firstname: 'Rayane',
  lastname: 'Guemmoud',
  username: 'rayane',
  birthdate: '2000-01-01',
  region: 'FR',
  regions: [
    {
      country_code: 'FR',
      data_region: 'eu-west',
      legal_jurisdiction: 'FR',
      primary_timezone: 'Europe/Paris',
      timezones: ['Europe/Paris'],
      sub_region: 'Europe',
      display_name: 'France',
    },
  ],
};

describe('validatePersonalInfo', () => {
  it('accepts a complete profile', () => {
    expect(validatePersonalInfo(validInput)).toEqual({});
  });

  it('reports every required invalid field', () => {
    expect(
      validatePersonalInfo({
        ...validInput,
        firstname: ' ',
        lastname: '',
        username: ' ',
        birthdate: '',
        region: '',
      }),
    ).toEqual({
      firstname: 'Le prénom est requis.',
      lastname: 'Le nom est requis.',
      username: 'Le nom d’utilisateur est requis.',
      birthdate: 'La date de naissance est requise.',
      region: 'La région est requise.',
    });
  });

  it('mirrors the backend username length limit', () => {
    expect(
      validatePersonalInfo({
        ...validInput,
        username: 'a'.repeat(101),
      }).username,
    ).toBe('Le nom d’utilisateur doit contenir 100 caractères maximum.');
  });
});
