import { CalendarIcon } from 'lucide-react';
import { useEffect, useMemo, useRef, useState } from 'react';
import type { Matcher } from 'react-day-picker';
import { Button } from '@/components/ui/button';
import { Calendar } from '@/components/ui/calendar';
import { Field, FieldLabel } from '@/components/ui/field';
import { Input } from '@/components/ui/input';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import {
  dateFromDisplayValue,
  dateFromInputValue,
  dateIsInRange,
  dateToInputValue,
  displayValueFromInputValue,
  formatBirthdateDisplayValue,
} from './birthdate';

export function BirthdateField({
  id,
  label = 'Date de naissance',
  max,
  min,
  onChange,
  onValidityChange,
  required = false,
  value,
  externalError,
}: {
  id: string;
  label?: string;
  max: string;
  min: string;
  onChange: (value: string) => void;
  onValidityChange?: (valid: boolean) => void;
  required?: boolean;
  value: string;
  externalError?: string;
}) {
  const inputRef = useRef<HTMLInputElement>(null);
  const lastEmittedValueRef = useRef<string | undefined>(undefined);
  const [pickerOpen, setPickerOpen] = useState(false);
  const [displayValue, setDisplayValue] = useState(() => displayValueFromInputValue(value));
  const [error, setError] = useState<string | null>(null);
  const selectedDate = useMemo(() => dateFromInputValue(value), [value]);
  const minDate = useMemo(() => dateFromInputValue(min), [min]);
  const maxDate = useMemo(() => dateFromInputValue(max), [max]);
  const disabledMatchers = useMemo<Matcher[]>(() => {
    const matchers: Matcher[] = [];

    if (minDate) {
      matchers.push({ before: minDate });
    }

    if (maxDate) {
      matchers.push({ after: maxDate });
    }

    return matchers;
  }, [maxDate, minDate]);

  useEffect(() => {
    if (value === lastEmittedValueRef.current) {
      lastEmittedValueRef.current = undefined;
      return;
    }

    setDisplayValue(displayValueFromInputValue(value));
  }, [value]);

  const emitChange = (nextValue: string) => {
    lastEmittedValueRef.current = nextValue;
    onChange(nextValue);
  };

  const updateFromDisplayValue = (nextValue: string, input: HTMLInputElement | null) => {
    const formattedValue = formatBirthdateDisplayValue(nextValue);
    setDisplayValue(formattedValue);
    setError(null);
    input?.setCustomValidity('');

    if (!formattedValue) {
      const message = required ? 'La date de naissance est requise.' : null;
      setError(message);
      onValidityChange?.(!required);
      emitChange('');
      return;
    }

    const date = dateFromDisplayValue(formattedValue);
    if (!date) {
      onValidityChange?.(false);
      if (formattedValue.length >= 10) {
        const message = 'Date de naissance invalide.';
        setError(message);
        emitChange('');
        input?.setCustomValidity(message);
      }
      return;
    }

    if (!dateIsInRange(date, minDate, maxDate)) {
      const message =
        'La date de naissance doit correspondre à un âge compris entre 13 et 120 ans.';
      setError(message);
      onValidityChange?.(false);
      emitChange('');
      input?.setCustomValidity(message);
      return;
    }

    onValidityChange?.(true);
    emitChange(dateToInputValue(date));
  };

  return (
    <Field>
      <FieldLabel htmlFor={id}>
        {label}
        {required && (
          <>
            {' '}
            <span className="text-destructive">*</span>
          </>
        )}
      </FieldLabel>
      <div className="relative">
        <Input
          id={id}
          ref={inputRef}
          type="text"
          placeholder="jj/mm/aaaa"
          value={displayValue}
          onChange={(event) => updateFromDisplayValue(event.target.value, event.currentTarget)}
          onBlur={(event) => updateFromDisplayValue(event.target.value, event.currentTarget)}
          inputMode="numeric"
          pattern="\d{2}/\d{2}/\d{4}"
          required={required}
          autoComplete="bday"
          aria-describedby={error || externalError ? `${id}-error` : undefined}
          aria-invalid={Boolean(error || externalError)}
          className="pr-10"
        />
        <Popover open={pickerOpen} onOpenChange={setPickerOpen}>
          <PopoverTrigger asChild>
            <Button
              type="button"
              variant="ghost"
              size="icon"
              className="absolute top-1/2 right-1 -translate-y-1/2"
              aria-label="Ouvrir le calendrier"
            >
              <CalendarIcon />
            </Button>
          </PopoverTrigger>
          <PopoverContent className="w-auto p-0" align="end">
            <Calendar
              mode="single"
              selected={selectedDate}
              onSelect={(date) => {
                if (!date) {
                  return;
                }

                inputRef.current?.setCustomValidity('');
                setError(null);
                onValidityChange?.(true);
                emitChange(dateToInputValue(date));
                setPickerOpen(false);
              }}
              captionLayout="dropdown"
              startMonth={minDate}
              endMonth={maxDate}
              disabled={disabledMatchers}
            />
          </PopoverContent>
        </Popover>
      </div>
      {(error || externalError) && (
        <p id={`${id}-error`} className="text-sm text-destructive" role="alert">
          {error ?? externalError}
        </p>
      )}
    </Field>
  );
}
