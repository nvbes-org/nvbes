import type { FormEvent } from 'react';
import type { EmailAddress } from '@nvbes/identity-client';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';

export function AccountEmailAddresses({
  emails,
  emailDraft,
  error,
  loading,
  primaryMinAgeHours,
  success,
  onAdd,
  onDelete,
  onDraftChange,
  onPromote,
  onResendVerification,
}: {
  emails: EmailAddress[];
  emailDraft: string;
  error: string | null;
  loading: boolean;
  primaryMinAgeHours: number;
  success: string | null;
  onAdd: (event: FormEvent<HTMLFormElement>) => void;
  onDelete: (emailId: string) => void;
  onDraftChange: (value: string) => void;
  onPromote: (emailId: string) => void;
  onResendVerification: (emailId: string) => void;
}) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>Emails</CardTitle>
        <CardDescription>
          L&apos;email principal reste celui utilise pour la connexion et les documents du compte.
        </CardDescription>
      </CardHeader>

      <CardContent className="flex flex-col gap-4">
        <div className="flex flex-col gap-2">
          {emails.map((email) => (
            <EmailAddressRow
              key={email.id}
              email={email}
              loading={loading}
              primaryMinAgeHours={primaryMinAgeHours}
              onDelete={onDelete}
              onPromote={onPromote}
              onResendVerification={onResendVerification}
            />
          ))}
        </div>

        <form
          onSubmit={onAdd}
          className="grid gap-3 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-end"
        >
          <div className="flex flex-col gap-2">
            <Label htmlFor="secondary-email">Ajouter un email secondaire</Label>
            <Input
              id="secondary-email"
              type="email"
              value={emailDraft}
              placeholder="adresse@email.com"
              autoComplete="email"
              onChange={(event) => onDraftChange(event.target.value)}
            />
          </div>
          <Button type="submit" disabled={loading || emailDraft.trim().length === 0}>
            Ajouter
          </Button>
        </form>

        {error ? <p className="text-sm text-destructive">{error}</p> : null}
        {success ? <p className="text-sm text-primary">{success}</p> : null}
      </CardContent>
    </Card>
  );
}

function EmailAddressRow({
  email,
  loading,
  primaryMinAgeHours,
  onDelete,
  onPromote,
  onResendVerification,
}: {
  email: EmailAddress;
  loading: boolean;
  primaryMinAgeHours: number;
  onDelete: (emailId: string) => void;
  onPromote: (emailId: string) => void;
  onResendVerification: (emailId: string) => void;
}) {
  const canPromote = !email.is_primary && email.verified && isOldEnough(email, primaryMinAgeHours);

  return (
    <div className="flex flex-col gap-3 rounded-lg border px-3 py-3 sm:flex-row sm:items-center sm:justify-between">
      <div className="min-w-0">
        <div className="flex flex-wrap items-center gap-2">
          <span className="truncate text-sm font-medium">{email.email}</span>
          {email.is_primary ? (
            <Badge>Principal</Badge>
          ) : (
            <Badge variant="secondary">Secondaire</Badge>
          )}
          <Badge variant={email.verified ? 'default' : 'secondary'}>
            {email.verified ? 'Verifie' : 'A verifier'}
          </Badge>
        </div>
        {!email.is_primary && email.verified && !canPromote ? (
          <p className="mt-1 text-xs text-muted-foreground">
            Vous pourrez le définir comme principal après {primaryMinAgeHours} h.
          </p>
        ) : null}
      </div>

      <div className="flex shrink-0 gap-2">
        {!email.is_primary ? (
          <>
            {!email.verified ? (
              <Button
                type="button"
                variant="outline"
                size="sm"
                disabled={loading}
                onClick={() => onResendVerification(email.id)}
              >
                Renvoyer
              </Button>
            ) : null}
            <Button
              type="button"
              variant="outline"
              size="sm"
              disabled={loading || !canPromote}
              onClick={() => onPromote(email.id)}
            >
              Principal
            </Button>
            <Button
              type="button"
              variant="destructive"
              size="sm"
              disabled={loading}
              onClick={() => onDelete(email.id)}
            >
              Supprimer
            </Button>
          </>
        ) : null}
      </div>
    </div>
  );
}

function isOldEnough(email: EmailAddress, primaryMinAgeHours: number): boolean {
  const createdAt = new Date(email.created_at).getTime();
  return (
    Number.isFinite(createdAt) && Date.now() - createdAt >= primaryMinAgeHours * 60 * 60 * 1000
  );
}
