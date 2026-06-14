import type { EnterpriseModuleGrant, EnterpriseRole } from '@nvbes/identity-client';
import { AlertTriangle } from 'lucide-react';
import { useMemo, useState } from 'react';
import { Alert, AlertDescription, AlertTitle } from '../components/ui/alert';
import { Button } from '../components/ui/button';
import { Checkbox } from '../components/ui/checkbox';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '../components/ui/dialog';
import {
  Field,
  FieldContent,
  FieldDescription,
  FieldError,
  FieldGroup,
  FieldLabel,
  FieldSet,
  FieldLegend,
} from '../components/ui/field';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '../components/ui/select';
import { Textarea } from '../components/ui/textarea';
import { buildEnterpriseInvitationInput, parseInvitationEmails } from '../enterprise.invites';
import { describeModuleGrant } from '../enterprise.permissions';

export type InviteSubmitValue = {
  emailInput: string;
  role: EnterpriseRole;
  module_grants: EnterpriseModuleGrant[];
  workspace_ids: string[];
};

type UsersPageInviteProps = {
  roles: EnterpriseRole[];
  grants: EnterpriseModuleGrant[];
  workspaceIds: string[];
  submitting: boolean;
  recipientErrors: Array<{ email: string; message: string }>;
  onSubmit: (value: InviteSubmitValue) => Promise<boolean>;
};

export function UsersPageInvite({
  roles,
  grants,
  workspaceIds,
  submitting,
  recipientErrors,
  onSubmit,
}: UsersPageInviteProps) {
  const [open, setOpen] = useState(false);
  const [emailInput, setEmailInput] = useState('');
  const [role, setRole] = useState<EnterpriseRole>(roles[0] ?? 'member');
  const [moduleGrants, setModuleGrants] = useState<EnterpriseModuleGrant[]>([]);
  const [workspaceAssignments, setWorkspaceAssignments] = useState<string[]>([]);
  const [formError, setFormError] = useState<string | null>(null);
  const recipients = useMemo(() => safeRecipients(emailInput), [emailInput]);

  async function submitForm(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setFormError(null);

    try {
      buildEnterpriseInvitationInput({
        emailInput,
        role,
        module_grants: moduleGrants,
        workspace_ids: workspaceAssignments,
      });
    } catch (error) {
      setFormError(error instanceof Error ? error.message : 'Invitation details are invalid.');
      return;
    }

    const success = await onSubmit({
      emailInput,
      role,
      module_grants: moduleGrants,
      workspace_ids: workspaceAssignments,
    });

    if (success) {
      setEmailInput('');
      setModuleGrants([]);
      setWorkspaceAssignments([]);
      setOpen(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button type="button">Invite users</Button>
      </DialogTrigger>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-xl">
        <form className="flex flex-col gap-5" onSubmit={submitForm}>
          <DialogHeader>
            <DialogTitle>Invite tenant users</DialogTitle>
            <DialogDescription>
              Send one or more invitations with initial role, module, and workspace access.
            </DialogDescription>
          </DialogHeader>

          <Field>
            <FieldLabel htmlFor="invite-emails">Email recipients</FieldLabel>
            <Textarea
              id="invite-emails"
              value={emailInput}
              onChange={(event) => setEmailInput(event.target.value)}
              placeholder="admin@example.com, analyst@example.com"
            />
            <FieldDescription>
              Separate multiple recipients with commas or new lines.
            </FieldDescription>
            {formError ? <FieldError>{formError}</FieldError> : null}
          </Field>

          <Field>
            <FieldLabel>Role</FieldLabel>
            <Select value={role} onValueChange={(nextRole) => setRole(nextRole as EnterpriseRole)}>
              <SelectTrigger className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {roles.map((availableRole) => (
                  <SelectItem key={availableRole} value={availableRole}>
                    {availableRole}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </Field>

          <CheckboxSet
            legend="Module grants"
            options={grants.map((grant) => ({ id: grant, label: describeModuleGrant(grant) }))}
            selected={moduleGrants}
            onChange={setModuleGrants}
          />

          <CheckboxSet
            legend="Workspace assignments"
            empty="Workspace IDs will appear after users or invitations include assignments."
            options={workspaceIds.map((id) => ({ id, label: id }))}
            selected={workspaceAssignments}
            onChange={setWorkspaceAssignments}
          />

          <Alert>
            <AlertTriangle className="size-4" />
            <AlertTitle>Invitation preview</AlertTitle>
            <AlertDescription>
              {recipients.length === 0
                ? 'No recipients parsed yet.'
                : `${recipients.length} recipient(s) will receive ${role} access with ${moduleGrants.length} module grant(s) and ${workspaceAssignments.length} workspace assignment(s).`}
            </AlertDescription>
          </Alert>

          {recipientErrors.length > 0 ? (
            <div className="rounded-lg border border-destructive/30 bg-destructive/5 p-3">
              <p className="text-sm font-medium text-destructive">Recipient errors</p>
              <ul className="mt-2 space-y-1 text-sm text-destructive">
                {recipientErrors.map((error) => (
                  <li key={error.email}>
                    {error.email}: {error.message}
                  </li>
                ))}
              </ul>
            </div>
          ) : null}

          <DialogFooter>
            <Button type="submit" disabled={submitting}>
              {submitting ? 'Sending...' : 'Send invitations'}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

function CheckboxSet<T extends string>({
  legend,
  empty = 'No options are available.',
  options,
  selected,
  onChange,
}: {
  legend: string;
  empty?: string;
  options: Array<{ id: T; label: string }>;
  selected: T[];
  onChange: (selected: T[]) => void;
}) {
  return (
    <FieldSet>
      <FieldLegend>{legend}</FieldLegend>
      {options.length === 0 ? (
        <FieldDescription>{empty}</FieldDescription>
      ) : (
        <FieldGroup data-slot="checkbox-group" className="gap-3">
          {options.map((option) => (
            <Field key={option.id} orientation="horizontal">
              <Checkbox
                checked={selected.includes(option.id)}
                onCheckedChange={(checked) => {
                  onChange(
                    checked
                      ? [...selected, option.id]
                      : selected.filter((item) => item !== option.id),
                  );
                }}
              />
              <FieldContent>
                <FieldLabel>{option.label}</FieldLabel>
              </FieldContent>
            </Field>
          ))}
        </FieldGroup>
      )}
    </FieldSet>
  );
}

function safeRecipients(emailInput: string): string[] {
  try {
    return parseInvitationEmails(emailInput);
  } catch {
    return [];
  }
}
