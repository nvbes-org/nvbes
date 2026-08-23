import type { CreateAccessReviewCampaignInput } from '@nvbes/identity-client';
import { ClipboardCheck } from 'lucide-react';
import { useState } from 'react';
import { Button } from '../components/ui/button';
import { Checkbox } from '../components/ui/checkbox';
import { Input } from '../components/ui/input';
import { Label } from '../components/ui/label';

const scopeOptions = [
  ['include_members', 'Members'],
  ['include_roles', 'Roles'],
  ['include_service_accounts', 'Service accounts'],
  ['include_oauth_clients', 'OAuth clients'],
] as const;

type ScopeKey = (typeof scopeOptions)[number][0];

const defaultScope: CreateAccessReviewCampaignInput['scope'] = {
  include_members: true,
  include_roles: true,
  include_service_accounts: true,
  include_oauth_clients: true,
};

export function CreateAccessReviewCampaignForm({
  error,
  pending,
  onSubmit,
}: {
  error: Error | null;
  pending: boolean;
  onSubmit: (input: CreateAccessReviewCampaignInput) => void;
}) {
  const [form, setForm] = useState(() => ({
    name: 'Quarterly access review',
    dueDate: defaultDueDate(),
    scope: defaultScope,
  }));
  const disabled = pending || !form.name.trim() || !hasScope(form.scope);

  return (
    <form
      className="flex flex-col gap-4"
      onSubmit={(event) => {
        event.preventDefault();
        onSubmit({
          name: form.name,
          due_at: new Date(`${form.dueDate}T23:59:00`).toISOString(),
          scope: form.scope,
        });
      }}
    >
      <div className="grid gap-2">
        <Label htmlFor="access-review-name">Name</Label>
        <Input
          id="access-review-name"
          value={form.name}
          onChange={(event) => setForm((current) => ({ ...current, name: event.target.value }))}
        />
      </div>
      <div className="grid gap-2">
        <Label htmlFor="access-review-due">Due date</Label>
        <Input
          id="access-review-due"
          type="date"
          value={form.dueDate}
          onChange={(event) => setForm((current) => ({ ...current, dueDate: event.target.value }))}
        />
      </div>
      <div className="grid gap-3">
        <Label>Scope</Label>
        {scopeOptions.map(([key, label]) => (
          <label key={key} className="flex items-center gap-2 text-sm">
            <Checkbox
              checked={form.scope[key]}
              onCheckedChange={(checked) =>
                setForm((current) => ({
                  ...current,
                  scope: { ...current.scope, [key]: checked === true },
                }))
              }
            />
            <span>{label}</span>
          </label>
        ))}
      </div>
      {error ? <p className="text-sm text-destructive">{error.message}</p> : null}
      <Button type="submit" disabled={disabled}>
        <ClipboardCheck className="size-4" />
        Start campaign
      </Button>
    </form>
  );
}

function defaultDueDate() {
  const due = new Date();
  due.setDate(due.getDate() + 14);
  return due.toISOString().slice(0, 10);
}

function hasScope(scope: Record<ScopeKey, boolean>) {
  return scopeOptions.some(([key]) => scope[key]);
}
