import type { ChangeEvent, FormEvent } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';

export function DeviceActivationInputStep({
  userCode,
  loading,
  onUserCodeChange,
  onSubmit,
}: {
  userCode: string;
  loading: boolean;
  onUserCodeChange: (value: string) => void;
  onSubmit: (event: FormEvent<HTMLFormElement>) => void;
}) {
  return (
    <form onSubmit={onSubmit} className="space-y-6">
      <div className="space-y-2">
        <Label htmlFor="userCode">Code d&apos;activation</Label>
        <Input
          id="userCode"
          placeholder="XXXX-XXXX"
          value={userCode}
          onChange={(event: ChangeEvent<HTMLInputElement>) =>
            onUserCodeChange(event.target.value.toUpperCase())
          }
          className="text-center font-mono text-2xl tracking-widest"
          required
        />
      </div>
      <Button type="submit" disabled={loading} className="h-12 w-full text-lg">
        {loading ? 'Vérification...' : 'Continuer'}
      </Button>
    </form>
  );
}
