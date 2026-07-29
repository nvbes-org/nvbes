import type { SupportedRegion } from '@/identity.auth.api';

export interface PersonalInfoFieldErrors {
  firstname?: string;
  lastname?: string;
  username?: string;
  birthdate?: string;
  region?: string;
}

export function validatePersonalInfo({
  firstname,
  lastname,
  username,
  birthdate,
  region,
  regions,
}: {
  firstname: string;
  lastname: string;
  username: string;
  birthdate: string;
  region: string;
  regions: SupportedRegion[];
}): PersonalInfoFieldErrors {
  const errors: PersonalInfoFieldErrors = {};

  if (!firstname.trim()) {
    errors.firstname = 'Le prénom est requis.';
  }
  if (!lastname.trim()) {
    errors.lastname = 'Le nom est requis.';
  }

  const trimmedUsername = username.trim();
  if (!trimmedUsername) {
    errors.username = 'Le nom d’utilisateur est requis.';
  } else if (trimmedUsername.length > 100) {
    errors.username = 'Le nom d’utilisateur doit contenir 100 caractères maximum.';
  }

  if (!birthdate) {
    errors.birthdate = 'La date de naissance est requise.';
  }

  if (!region) {
    errors.region = 'La région est requise.';
  } else if (regions.length > 0 && !regions.some((entry) => entry.country_code === region)) {
    errors.region = 'Sélectionnez une région proposée dans la liste.';
  }

  return errors;
}

export function hasPersonalInfoErrors(errors: PersonalInfoFieldErrors): boolean {
  return Object.values(errors).some(Boolean);
}
