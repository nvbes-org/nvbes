import { ArrowLeftIcon, CheckIcon } from 'lucide-react';
import type { FormEvent, ReactNode } from 'react';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Spinner } from '@/components/ui/spinner';
import type { SupportedRegion } from '../identity.auth.api';

export function RegisterPageStepTwo({
  workspaceName,
  selectedRegion,
  detectedRegion,
  detectedReliability,
  regionLoading,
  supportedRegions,
  error,
  loading,
  onWorkspaceNameChange,
  onRegionChange,
  onBack,
  onSubmit,
  regionSelect,
}: {
  workspaceName: string;
  selectedRegion: string;
  detectedRegion: string | null;
  detectedReliability: 'high' | 'medium' | 'low' | 'none';
  regionLoading: boolean;
  supportedRegions: SupportedRegion[];
  error: string | null;
  loading: boolean;
  onWorkspaceNameChange: (value: string) => void;
  onRegionChange: (value: string) => void;
  onBack: () => void;
  onSubmit: (event: FormEvent<HTMLFormElement>) => void;
  regionSelect: (props: {
    detectedRegion: string | null;
    reliability: 'high' | 'medium' | 'low' | 'none';
    loading: boolean;
    regions: SupportedRegion[];
    value: string;
    onValueChange: (value: string) => void;
  }) => ReactNode;
}) {
  return (
    <form onSubmit={onSubmit} className="flex flex-col gap-5">
      <div className="flex flex-col gap-2">
        <Label htmlFor="register-workspace">Nom du workspace</Label>
        <Input
          id="register-workspace"
          type="text"
          placeholder="Mon workspace"
          value={workspaceName}
          onChange={(event) => onWorkspaceNameChange(event.target.value)}
          required
          autoFocus
        />
      </div>

      {regionSelect({
        detectedRegion,
        reliability: detectedReliability,
        loading: regionLoading,
        regions: supportedRegions,
        value: selectedRegion,
        onValueChange: onRegionChange,
      })}

      {error && (
        <Alert variant="destructive">
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}

      <div className="flex gap-2">
        <Button
          type="button"
          variant="outline"
          className="flex-1"
          onClick={onBack}
          disabled={loading}
        >
          <ArrowLeftIcon data-icon="inline-start" />
          Retour
        </Button>
        <Button type="submit" className="flex-1" disabled={loading}>
          {loading ? (
            <>
              <Spinner data-icon="inline-start" />
              Inscription...
            </>
          ) : (
            <>
              S&apos;inscrire
              <CheckIcon data-icon="inline-end" />
            </>
          )}
        </Button>
      </div>
    </form>
  );
}
