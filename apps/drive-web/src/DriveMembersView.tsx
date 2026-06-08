import { MailPlus, ShieldCheck, UserRound } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { inviteMember, setMemberRole } from './drive.workspace.store';
import type { DriveRole, DriveWorkspaceState } from './drive.workspace.types';

const INVITE_EMAIL = 'invite@studio.test';
const ASSIGNABLE_ROLE_OPTIONS: Array<Exclude<DriveRole, 'owner'>> = ['admin', 'member', 'viewer'];

export function DriveMembersView({
  state,
  onStateChange,
}: {
  state: DriveWorkspaceState;
  onStateChange: (state: DriveWorkspaceState) => void;
}) {
  function handleInvite() {
    const invitedEmailExists = state.members.some(
      (member) => member.email.toLocaleLowerCase() === INVITE_EMAIL,
    );

    if (invitedEmailExists) {
      onStateChange({
        ...state,
        toast: {
          id: `toast-invite-existing-${INVITE_EMAIL}`,
          message: 'Invitation deja presente',
          tone: 'info',
        },
      });
      return;
    }

    onStateChange(inviteMember(state, INVITE_EMAIL, 'member'));
  }

  function handleRoleChange(memberId: string, role: DriveRole) {
    onStateChange(setMemberRole(state, memberId, role));
  }

  return (
    <section className="grid gap-4">
      <div className="flex flex-col gap-3 rounded-2xl border border-border/70 bg-background p-4 shadow-sm sm:flex-row sm:items-center sm:justify-between">
        <div>
          <p className="text-xs font-semibold uppercase tracking-[0.2em] text-muted-foreground">
            Administration
          </p>
          <h2 className="mt-1 text-lg font-semibold">Membres du workspace</h2>
          <p className="text-sm text-muted-foreground">
            Invitez les collaborateurs et ajustez leurs roles Drive.
          </p>
        </div>
        <Button type="button" onClick={handleInvite}>
          <MailPlus className="size-4" aria-hidden="true" />
          Inviter {INVITE_EMAIL}
        </Button>
      </div>

      <div className="grid gap-3">
        {state.members.map((member) => {
          const isOwner = member.role === 'owner';

          return (
            <article
              key={member.id}
              className="rounded-2xl border border-border/70 bg-background p-4 shadow-sm"
            >
              <div className="flex flex-col gap-4 lg:flex-row lg:items-center lg:justify-between">
                <div className="flex min-w-0 items-start gap-3">
                  <span className="grid size-10 shrink-0 place-items-center rounded-2xl bg-muted text-primary">
                    <UserRound className="size-5" aria-hidden="true" />
                  </span>
                  <div className="min-w-0">
                    <h3 className="truncate font-medium">{member.name}</h3>
                    <p className="truncate text-sm text-muted-foreground">{member.email}</p>
                  </div>
                </div>

                <dl className="grid gap-3 text-sm sm:grid-cols-2 lg:min-w-[320px]">
                  <div>
                    <dt className="text-xs uppercase tracking-wide text-muted-foreground">Statut</dt>
                    <dd className="font-medium">{memberStatusLabel(member.status)}</dd>
                  </div>
                  <div>
                    <dt className="text-xs uppercase tracking-wide text-muted-foreground">Role</dt>
                    <dd className="font-medium">{roleLabel(member.role)}</dd>
                  </div>
                </dl>

                <label className="grid gap-1 text-sm lg:min-w-44">
                  <span className="text-xs uppercase tracking-wide text-muted-foreground">
                    Changer le role
                  </span>
                  <select
                    className="h-9 rounded-lg border border-border bg-background px-2 text-sm outline-none transition focus:border-ring focus:ring-2 focus:ring-ring/30 disabled:cursor-not-allowed disabled:opacity-50"
                    value={member.role}
                    disabled={isOwner}
                    title={isOwner ? 'Le proprietaire ne peut pas etre modifie ici.' : undefined}
                    onChange={(event) => handleRoleChange(member.id, event.target.value as DriveRole)}
                  >
                    {isOwner ? <option value="owner">{roleLabel('owner')}</option> : null}
                    {ASSIGNABLE_ROLE_OPTIONS.map((role) => (
                      <option key={role} value={role}>
                        {roleLabel(role)}
                      </option>
                    ))}
                  </select>
                  {isOwner ? (
                    <span className="flex items-center gap-1 text-xs text-muted-foreground">
                      <ShieldCheck className="size-3" aria-hidden="true" />
                      Proprietaire non modifiable
                    </span>
                  ) : null}
                </label>
              </div>
            </article>
          );
        })}
      </div>
    </section>
  );
}

function roleLabel(role: DriveRole): string {
  if (role === 'owner') return 'Proprietaire';
  if (role === 'admin') return 'Admin';
  if (role === 'member') return 'Membre';
  return 'Lecteur';
}

function memberStatusLabel(status: DriveWorkspaceState['members'][number]['status']): string {
  if (status === 'active') return 'Actif';
  if (status === 'invited') return 'Invite';
  return 'Suspendu';
}
