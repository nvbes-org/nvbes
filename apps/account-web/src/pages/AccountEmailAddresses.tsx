import type { SubmitEvent } from 'react';
import type { EmailAddress } from '@nvbes/identity-client';
import { MailPlus, Send, Trash2 } from 'lucide-react';

import { AsyncStateButton } from '@/components/AsyncStateButton';
import { Badge } from '@/components/ui/badge';
import { Card, CardContent } from '@/components/ui/card';
import { Field, FieldLabel } from '@/components/ui/field';
import { Input } from '@/components/ui/input';

export function AccountEmailAddresses({
  adding,
  deletingEmailId,
  emails,
  emailDraft,
  error,
  promotingEmailId,
  primaryMinAgeHours,
  resendingEmailId,
  onAdd,
  onDelete,
  onDraftChange,
  onPromote,
  onResendVerification,
}: {
  adding: boolean;
  deletingEmailId: string | null;
  emails: EmailAddress[];
  emailDraft: string;
  error: string | null;
  promotingEmailId: string | null;
  primaryMinAgeHours: number;
  resendingEmailId: string | null;
  onAdd: (event: SubmitEvent<HTMLFormElement>) => void;
  onDelete: (emailId: string) => void;
  onDraftChange: (value: string) => void;
  onPromote: (emailId: string) => void;
  onResendVerification: (emailId: string) => void;
}) {
  return (
    <Card className="flex flex-col gap-4">
      <CardContent>
        <form
          onSubmit={onAdd}
          className="grid gap-3 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-end pb-5"
        >
          <Field>
            <FieldLabel htmlFor="secondary-email">Ajouter un email secondaire</FieldLabel>
            <Input
              id="secondary-email"
              type="email"
              value={emailDraft}
              placeholder="adresse@email.com"
              autoComplete="email"
              onChange={(event) => onDraftChange(event.target.value)}
              className="bg-card"
            />
          </Field>
          <AsyncStateButton
            type="submit"
            size="icon"
            disabled={adding || emailDraft.trim().length === 0}
            state={adding ? 'pending' : 'idle'}
            message="Ajouter l’adresse email secondaire"
            icon={<MailPlus />}
          />
        </form>
        <div className="flex flex-col gap-2">
          {emails
            .sort((email) => (email.is_primary ? 1 : 0))
            .map((email) => (
              <EmailAddressRow
                key={email.id}
                email={email}
                deleting={deletingEmailId === email.id}
                promoting={promotingEmailId === email.id}
                primaryMinAgeHours={primaryMinAgeHours}
                resending={resendingEmailId === email.id}
                onDelete={onDelete}
                onPromote={onPromote}
                onResendVerification={onResendVerification}
              />
            ))}
        </div>

        {error ? <p className="text-sm text-destructive">{error}</p> : null}
      </CardContent>
    </Card>
  );
}

function EmailAddressRow({
  deleting,
  email,
  promoting,
  primaryMinAgeHours,
  resending,
  onDelete,
  onPromote,
  onResendVerification,
}: {
  deleting: boolean;
  email: EmailAddress;
  promoting: boolean;
  primaryMinAgeHours: number;
  resending: boolean;
  onDelete: (emailId: string) => void;
  onPromote: (emailId: string) => void;
  onResendVerification: (emailId: string) => void;
}) {
  const canPromote = !email.is_primary && email.verified && isOldEnough(email, primaryMinAgeHours);

  return (
    <div className="flex flex-col gap-3 rounded-lg border px-3 py-3 sm:flex-row sm:items-center sm:justify-between bg-card">
      <div className="min-w-0">
        <div className="flex flex-wrap items-center gap-2">
          <span className="truncate text-sm font-medium">{email.email}</span>
          {email.is_primary ? (
            <Badge>Principal</Badge>
          ) : (
            <Badge variant="secondary">Secondaire</Badge>
          )}
          <Badge variant={email.verified ? 'default' : 'destructive'}>
            {email.verified ? 'Vérifié' : 'Pas vérifié'}
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
            <AsyncStateButton
              type="button"
              variant="outline"
              size="sm"
              disabled={promoting || !canPromote}
              state={promoting ? 'pending' : canPromote ? 'idle' : 'disabled'}
              message="Définir comme principal"
              onClick={() => onPromote(email.id)}
            />
            {!email.verified ? (
              <AsyncStateButton
                type="button"
                variant="outline"
                size="icon-sm"
                disabled={resending}
                state={resending ? 'pending' : 'idle'}
                message={`Renvoyer l’email de vérification à ${email.email}`}
                icon={<Send />}
                onClick={() => onResendVerification(email.id)}
              />
            ) : null}
            <AsyncStateButton
              type="button"
              variant="destructive"
              size="icon-sm"
              disabled={deleting}
              state={deleting ? 'pending' : 'idle'}
              message={`Supprimer l’adresse ${email.email}`}
              icon={<Trash2 />}
              onClick={() => onDelete(email.id)}
            />
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
