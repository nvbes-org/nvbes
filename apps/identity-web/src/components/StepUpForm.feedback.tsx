import { ShieldAlert } from 'lucide-react';

import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import type { StepUpMethod } from './useStepUpForm';

export function StepUpError({ error }: { error: string }) {
  return (
    <Alert variant="destructive">
      <ShieldAlert />
      <AlertTitle>Vérification impossible</AlertTitle>
      <AlertDescription>{error}</AlertDescription>
    </Alert>
  );
}

export function StepUpActions({
  method,
  loading,
  onCancel,
}: {
  method: StepUpMethod;
  loading: boolean;
  onCancel: () => void;
}) {
  return (
    <div className="flex gap-2 pt-2">
      <Button
        type="button"
        variant="outline"
        className="flex-1 text-xs"
        onClick={onCancel}
        disabled={loading}
      >
        Annuler
      </Button>
      {method !== 'webauthn' && (
        <Button type="submit" className="flex-1 text-xs" disabled={loading}>
          {loading ? 'Vérification...' : 'Confirmer'}
        </Button>
      )}
    </div>
  );
}
