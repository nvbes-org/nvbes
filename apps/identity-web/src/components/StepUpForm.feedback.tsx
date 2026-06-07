import { ShieldAlert } from 'lucide-react';

import { Button } from '@/components/ui/button';
import type { StepUpMethod } from './useStepUpForm';

export function StepUpError({ error }: { error: string }) {
  return (
    <div className="flex items-center gap-2 rounded-lg bg-destructive/10 p-3 text-xs text-destructive">
      <ShieldAlert className="size-4 shrink-0" />
      <span>{error}</span>
    </div>
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
