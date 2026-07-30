import type { ReactNode } from 'react';
import { FeedbackAlert } from '@/components/FeedbackAlert';
import { SecuritySetupActions, SecuritySetupPanel } from '@/components/SecuritySetupPanel';
import { Button } from '@/components/ui/button';
import { Field, FieldLabel } from '@/components/ui/field';
import { Input } from '@/components/ui/input';

export function SecuritySetupForm({
  title,
  fieldId,
  fieldLabel,
  fieldPlaceholder,
  value,
  error,
  loading,
  submitLabel,
  pendingLabel,
  submitDisabled = false,
  children,
  onValueChange,
  onCancel,
  onSubmit,
}: {
  title: string;
  fieldId: string;
  fieldLabel: string;
  fieldPlaceholder: string;
  value: string;
  error: string | null;
  loading: boolean;
  submitLabel: string;
  pendingLabel: string;
  submitDisabled?: boolean;
  children?: ReactNode;
  onValueChange: (value: string) => void;
  onCancel: () => void;
  onSubmit: () => void;
}) {
  return (
    <SecuritySetupPanel title={title}>
      <Field>
        <FieldLabel htmlFor={fieldId}>{fieldLabel}</FieldLabel>
        <Input
          id={fieldId}
          placeholder={fieldPlaceholder}
          value={value}
          onChange={(event) => onValueChange(event.target.value)}
        />
      </Field>
      {children}
      {error ? <FeedbackAlert tone="error">{error}</FeedbackAlert> : null}
      <SecuritySetupActions>
        <Button variant="outline" className="flex-1" onClick={onCancel}>
          Annuler
        </Button>
        <Button className="flex-1" disabled={loading || submitDisabled} onClick={onSubmit}>
          {loading ? pendingLabel : submitLabel}
        </Button>
      </SecuritySetupActions>
    </SecuritySetupPanel>
  );
}
