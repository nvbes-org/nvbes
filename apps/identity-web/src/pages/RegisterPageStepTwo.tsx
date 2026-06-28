import { ArrowLeftIcon, CheckIcon } from 'lucide-react';
import type { FormEvent, ReactNode } from 'react';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
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
  legalDocumentsAccepted,
  loading,
  marketingEmailsAccepted,
  onWorkspaceNameChange,
  onLegalDocumentsAcceptedChange,
  onMarketingEmailsAcceptedChange,
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
  legalDocumentsAccepted: boolean;
  loading: boolean;
  marketingEmailsAccepted: boolean;
  onWorkspaceNameChange: (value: string) => void;
  onLegalDocumentsAcceptedChange: (value: boolean) => void;
  onMarketingEmailsAcceptedChange: (value: boolean) => void;
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
        <Label htmlFor="register-workspace">
          Nom du workspace <span className="text-destructive">*</span>
        </Label>
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

      <div className="flex flex-col gap-3 border-t border-border pt-4">
        <div className="flex items-start gap-3 text-sm leading-5">
          <Checkbox
            id="register-legal-documents"
            checked={legalDocumentsAccepted}
            onCheckedChange={(checked) => onLegalDocumentsAcceptedChange(checked === true)}
            className="mt-0.5"
          />
          <label htmlFor="register-legal-documents">
            J&apos;accepte les{' '}
            <a href="/legal/terms-of-service" className="font-medium text-primary hover:underline">
              Conditions
            </a>
            , la{' '}
            <a href="/legal/privacy-policy" className="font-medium text-primary hover:underline">
              Confidentialité
            </a>{' '}
            et le{' '}
            <a
              href="/legal/data-processing-agreement"
              className="font-medium text-primary hover:underline"
            >
              DPA
            </a>
            . <span className="text-destructive">*</span>
          </label>
        </div>
        <div className="flex items-start gap-3 text-sm leading-5 text-muted-foreground">
          <Checkbox
            id="register-marketing-emails"
            checked={marketingEmailsAccepted}
            onCheckedChange={(checked) => onMarketingEmailsAcceptedChange(checked === true)}
            className="mt-0.5"
          />
          <label htmlFor="register-marketing-emails">
            Je souhaite recevoir les nouveautés et conseils nvbes par email.
          </label>
        </div>
      </div>

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
        <Button type="submit" className="flex-1" disabled={loading || !legalDocumentsAccepted}>
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
