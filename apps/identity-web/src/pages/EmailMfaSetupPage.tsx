import { listEmailMfaEligible, setupEmailMfa } from '@nvbes/identity-sdk-web';
import { useNavigate } from '@tanstack/react-router';
import { Mail } from 'lucide-react';
import { useEffect, useState } from 'react';

import StepUpForm from '@/components/StepUpForm';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';

type EligibleEmail = {
  id: string;
  email: string;
  is_primary: boolean;
  verified: boolean;
  created_at: string;
};

export default function EmailMfaSetupPage() {
  const navigate = useNavigate();
  const [step, setStep] = useState<'stepup' | 'select' | 'done'>('stepup');
  const [emails, setEmails] = useState<EligibleEmail[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (step !== 'select') return;
    setLoading(true);
    listEmailMfaEligible('')
      .then((result) => {
        setEmails(result.emails);
        setError(null);
      })
      .catch((err: unknown) => {
        setError(err instanceof Error ? err.message : 'Impossible de charger les emails.');
      })
      .finally(() => setLoading(false));
  }, [step]);

  const navigateBack = () => void navigate({ to: '/account/mfa' });

  if (step === 'stepup') {
    return (
      <div className="flex min-h-screen items-center justify-center">
        <StepUpForm
          onSuccess={() => setStep('select')}
          onCancel={navigateBack}
          description="Pour activer la MFA par email, veuillez confirmer votre identite."
        />
      </div>
    );
  }

  if (step === 'done') {
    return (
      <div className="mx-auto flex max-w-xl flex-col gap-4">
        <h1 className="text-xl font-heading font-semibold">MFA email activee</h1>
        <p className="text-sm text-muted-foreground">
          Cet email pourra maintenant recevoir des codes de connexion.
        </p>
        <Button onClick={navigateBack}>Retour aux methodes MFA</Button>
      </div>
    );
  }

  return (
    <div className="mx-auto flex max-w-xl flex-col gap-5">
      <div>
        <h1 className="text-xl font-heading font-semibold">MFA par email</h1>
        <p className="mt-1 text-sm text-muted-foreground">
          Choisissez un email verifie et eligible pour recevoir vos codes de connexion.
        </p>
      </div>

      {error ? (
        <Alert variant="destructive">
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      ) : null}

      <div className="flex flex-col gap-2">
        {emails.map((email) => (
          <Button
            key={email.id}
            type="button"
            variant="outline"
            className="h-auto justify-start gap-3 py-3"
            disabled={loading}
            onClick={async () => {
              setLoading(true);
              setError(null);
              try {
                await setupEmailMfa('', email.id);
                setStep('done');
              } catch (err) {
                setError(err instanceof Error ? err.message : 'Activation impossible.');
              } finally {
                setLoading(false);
              }
            }}
          >
            <Mail data-icon="inline-start" />
            <span className="flex min-w-0 flex-col items-start">
              <span className="truncate">{email.email}</span>
              <span className="text-xs text-muted-foreground">
                {email.is_primary ? 'Email principal' : 'Email secondaire'}
              </span>
            </span>
          </Button>
        ))}
      </div>

      {!loading && emails.length === 0 ? (
        <p className="text-sm text-muted-foreground">
          Aucun email secondaire eligible. Verifiez un email secondaire et attendez la fenetre de
          securite requise.
        </p>
      ) : null}

      <Button type="button" variant="outline" onClick={navigateBack}>
        Annuler
      </Button>
    </div>
  );
}
