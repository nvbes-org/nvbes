import { Button } from '@/components/ui/button';
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import type { VerificationStatus } from './VerifyEmailPage.shared';
import { VerifyEmailStatusBanner } from './VerifyEmailPage.banner';

export function VerifyEmailCard({
  verified,
  message,
  status,
  emailDraft,
  emailLocked,
  canSend,
  working,
  emailChanged,
  resendInSeconds,
  onEmailDraftChange,
  onAction,
  onBackToLogin,
}: {
  verified: boolean;
  message: string | null;
  status: VerificationStatus;
  emailDraft: string;
  emailLocked: boolean;
  canSend: boolean;
  working: boolean;
  emailChanged: boolean;
  resendInSeconds: number;
  onEmailDraftChange: (value: string) => void;
  onAction: () => void;
  onBackToLogin: () => void;
}) {
  return (
    <div className="flex min-h-screen items-center justify-center bg-background px-4 py-10 sm:px-6">
      <Card className="w-full max-w-md">
        <CardHeader>
          <CardTitle>Vérifie ton email</CardTitle>
          <CardDescription>
            {verified ? 'Compte vérifié.' : 'Confirme ton adresse email.'}
          </CardDescription>
        </CardHeader>

        <CardContent className="flex flex-col gap-5">
          {message ? <VerifyEmailStatusBanner status={status} message={message} /> : null}

          <div className="flex flex-col gap-3">
            <Label htmlFor="verification-email">Email du compte</Label>
            <Input
              id="verification-email"
              type="email"
              value={emailDraft}
              onChange={(event) => onEmailDraftChange(event.target.value)}
              autoComplete="email"
              disabled={emailLocked}
              placeholder="adresse@email.com"
            />
          </div>
        </CardContent>

        <CardFooter className="flex flex-col gap-3 sm:flex-row">
          <Button
            type="button"
            variant="outline"
            className="w-full sm:flex-1"
            onClick={onBackToLogin}
          >
            Retour à la connexion
          </Button>
          {!verified ? (
            <Button
              type="button"
              className="w-full sm:flex-1"
              onClick={onAction}
              disabled={!canSend}
            >
              {working
                ? 'Envoi...'
                : emailChanged
                  ? 'Mettre à jour et renvoyer'
                  : resendInSeconds > 0
                    ? `Renvoyer dans ${resendInSeconds}s`
                    : 'Renvoyer la vérification'}
            </Button>
          ) : (
            <Button type="button" className="w-full sm:flex-1" disabled>
              Vérifié
            </Button>
          )}
        </CardFooter>
      </Card>
    </div>
  );
}
