const MINIMUM_ACCOUNT_AGE = 13;
const MAXIMUM_ACCOUNT_AGE = 120;

export function dateFromInputValue(value: string): Date | undefined {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/u.exec(value);
  if (!match) {
    return undefined;
  }

  const [, year, month, day] = match;
  const date = new Date(0);
  date.setHours(0, 0, 0, 0);
  date.setFullYear(Number(year), Number(month) - 1, Number(day));

  if (date.getFullYear() !== Number(year) || date.getMonth() !== Number(month) - 1 || date.getDate() !== Number(day)) {
    return undefined;
  }

  return date;
}

export function dateToInputValue(date: Date): string {
  const year = date.getFullYear().toString().padStart(4, '0');
  const month = (date.getMonth() + 1).toString().padStart(2, '0');
  const day = date.getDate().toString().padStart(2, '0');

  return `${year}-${month}-${day}`;
}

export function displayValueFromInputValue(value: string): string {
  const date = dateFromInputValue(value);
  if (!date) {
    return '';
  }

  const day = date.getDate().toString().padStart(2, '0');
  const month = (date.getMonth() + 1).toString().padStart(2, '0');

  return `${day}/${month}/${date.getFullYear().toString().padStart(4, '0')}`;
}

export function dateFromDisplayValue(value: string): Date | undefined {
  const match = /^(\d{2})\/(\d{2})\/(\d{4})$/u.exec(value);
  if (!match) {
    return undefined;
  }

  const [, day, month, year] = match;
  return dateFromInputValue(`${year}-${month}-${day}`);
}

export function formatBirthdateDisplayValue(value: string): string {
  const digits = value.replace(/\D/gu, '').slice(0, 8);
  const day = digits.slice(0, 2);
  const month = digits.slice(2, 4);
  const year = digits.slice(4, 8);

  return [day, month, year].filter(Boolean).join('/');
}

export function dateIsInRange(date: Date, min: Date | undefined, max: Date | undefined): boolean {
  return (!min || date >= min) && (!max || date <= max);
}

export function birthdateBounds(referenceDate = new Date()): {
  minBirthdate: string;
  maxBirthdate: string;
} {
  const minDate = new Date(referenceDate);
  minDate.setFullYear(minDate.getFullYear() - MAXIMUM_ACCOUNT_AGE);

  const maxDate = new Date(referenceDate);
  maxDate.setFullYear(maxDate.getFullYear() - MINIMUM_ACCOUNT_AGE);

  return {
    minBirthdate: dateToInputValue(minDate),
    maxBirthdate: dateToInputValue(maxDate),
  };
}
