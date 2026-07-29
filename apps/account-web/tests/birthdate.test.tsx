import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vite-plus/test';
import { BirthdateField } from '../src/components/BirthdateField';
import {
  birthdateBounds,
  dateFromDisplayValue,
  dateFromInputValue,
  dateIsInRange,
  displayValueFromInputValue,
  formatBirthdateDisplayValue,
} from '../src/components/birthdate';

describe('birthdate', () => {
  it('formats a French date entry without inventing a leading zero for the year', () => {
    expect(formatBirthdateDisplayValue('09/01/800')).toBe('09/01/800');
  });

  it('requires a four-digit year', () => {
    expect(dateFromDisplayValue('09/01/800')).toBeUndefined();
    expect(dateFromDisplayValue('09/01/0800')?.getFullYear()).toBe(800);
  });

  it('rejects a padded ancient year through the account age limits', () => {
    const date = dateFromDisplayValue('09/01/0800');

    expect(date).toBeDefined();
    if (!date) {
      throw new Error('Expected a syntactically valid date.');
    }

    expect(dateIsInRange(date, dateFromInputValue('1906-07-29'), dateFromInputValue('2013-07-29'))).toBe(false);
  });

  it('formats the persisted ISO value for display', () => {
    expect(displayValueFromInputValue('2000-01-09')).toBe('09/01/2000');
  });

  it('uses the account age policy for date limits', () => {
    expect(birthdateBounds(new Date(2026, 6, 29))).toEqual({
      minBirthdate: '1906-07-29',
      maxBirthdate: '2013-07-29',
    });
  });

  it('renders an accessible numeric day/month/year field', () => {
    const markup = renderToStaticMarkup(
      <BirthdateField id="birthdate" value="2000-01-09" min="1906-07-29" max="2013-07-29" onChange={() => undefined} />,
    );

    expect(markup).toContain('type="text"');
    expect(markup).toContain('inputMode="numeric"');
    expect(markup).toContain('pattern="\\d{2}/\\d{2}/\\d{4}"');
    expect(markup).toContain('autoComplete="bday"');
    expect(markup).toContain('value="09/01/2000"');
  });
});
