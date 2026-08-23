import type { AccessReviewSchedule, CreateAccessReviewScheduleInput } from '@nvbes/identity-client';
import { CalendarClock, RefreshCw } from 'lucide-react';
import { useState } from 'react';
import { Alert, AlertDescription, AlertTitle } from '../components/ui/alert';
import { Button } from '../components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '../components/ui/card';
import { Checkbox } from '../components/ui/checkbox';
import { Input } from '../components/ui/input';
import { Label } from '../components/ui/label';
import { Skeleton } from '../components/ui/skeleton';
import { AccessReviewSchedulesTable } from './AccessReviewsPage.schedulesTable';

const scopeOptions = [
  ['include_members', 'Members'],
  ['include_roles', 'Roles'],
  ['include_service_accounts', 'Service accounts'],
  ['include_oauth_clients', 'OAuth clients'],
] as const;

type ScopeKey = (typeof scopeOptions)[number][0];

const defaultScope: CreateAccessReviewScheduleInput['scope'] = {
  include_members: true,
  include_roles: true,
  include_service_accounts: true,
  include_oauth_clients: true,
};

export function AccessReviewSchedulesPanel({
  error,
  createError,
  pending,
  createPending,
  mutatingScheduleId,
  schedules,
  runError,
  updateError,
  onRefresh,
  onCreate,
  onDisable,
  onEnable,
  onRunNow,
}: {
  error: Error | null;
  createError: Error | null;
  pending: boolean;
  createPending: boolean;
  mutatingScheduleId: string | null;
  schedules: AccessReviewSchedule[];
  runError: Error | null;
  updateError: Error | null;
  onRefresh: () => void;
  onCreate: (input: CreateAccessReviewScheduleInput) => void;
  onDisable: (scheduleId: string) => void;
  onEnable: (scheduleId: string) => void;
  onRunNow: (scheduleId: string) => void;
}) {
  return (
    <Card className="rounded-lg" size="sm">
      <CardHeader className="flex flex-row items-center justify-between gap-3">
        <CardTitle>Recurring schedules</CardTitle>
        <Button type="button" variant="outline" size="sm" onClick={onRefresh}>
          <RefreshCw className="size-3.5" />
          Refresh
        </Button>
      </CardHeader>
      <CardContent className="grid gap-4 xl:grid-cols-[360px_minmax(0,1fr)]">
        <CreateScheduleForm error={createError} pending={createPending} onSubmit={onCreate} />
        <div>
          {updateError ? (
            <Alert variant="destructive" className="mb-4">
              <AlertTitle>Schedule update failed</AlertTitle>
              <AlertDescription>{updateError.message}</AlertDescription>
            </Alert>
          ) : null}
          {runError ? (
            <Alert variant="destructive" className="mb-4">
              <AlertTitle>Schedule run failed</AlertTitle>
              <AlertDescription>{runError.message}</AlertDescription>
            </Alert>
          ) : null}
          {error ? (
            <Alert variant="destructive" className="mb-4">
              <AlertTitle>Schedules failed to load</AlertTitle>
              <AlertDescription>{error.message}</AlertDescription>
            </Alert>
          ) : null}
          {pending ? (
            <Skeleton className="h-48 w-full rounded-lg" />
          ) : (
            <AccessReviewSchedulesTable
              mutatingScheduleId={mutatingScheduleId}
              schedules={schedules}
              onDisable={onDisable}
              onEnable={onEnable}
              onRunNow={onRunNow}
            />
          )}
        </div>
      </CardContent>
    </Card>
  );
}

function CreateScheduleForm({
  error,
  pending,
  onSubmit,
}: {
  error: Error | null;
  pending: boolean;
  onSubmit: (input: CreateAccessReviewScheduleInput) => void;
}) {
  const [form, setForm] = useState(() => ({
    name: 'Quarterly access review',
    recurrenceDays: 90,
    dueAfterDays: 14,
    scope: defaultScope,
  }));
  const disabled =
    pending ||
    !form.name.trim() ||
    form.recurrenceDays < 7 ||
    form.dueAfterDays < 1 ||
    form.dueAfterDays > form.recurrenceDays ||
    !hasScope(form.scope);

  return (
    <form
      className="grid gap-4"
      onSubmit={(event) => {
        event.preventDefault();
        onSubmit({
          name: form.name,
          recurrence_days: form.recurrenceDays,
          due_after_days: form.dueAfterDays,
          scope: form.scope,
        });
      }}
    >
      <div className="grid gap-2">
        <Label htmlFor="access-review-schedule-name">Name</Label>
        <Input
          id="access-review-schedule-name"
          value={form.name}
          onChange={(event) => setForm((current) => ({ ...current, name: event.target.value }))}
        />
      </div>
      <div className="grid grid-cols-2 gap-3">
        <NumberField
          id="access-review-recurrence"
          label="Every days"
          value={form.recurrenceDays}
          onChange={(recurrenceDays) => setForm((current) => ({ ...current, recurrenceDays }))}
        />
        <NumberField
          id="access-review-due-after"
          label="Due after"
          value={form.dueAfterDays}
          onChange={(dueAfterDays) => setForm((current) => ({ ...current, dueAfterDays }))}
        />
      </div>
      <ScopeCheckboxes
        scope={form.scope}
        onChange={(scope) => setForm((current) => ({ ...current, scope }))}
      />
      {error ? <p className="text-sm text-destructive">{error.message}</p> : null}
      <Button type="submit" disabled={disabled}>
        <CalendarClock className="size-4" />
        Create schedule
      </Button>
    </form>
  );
}

function NumberField({
  id,
  label,
  value,
  onChange,
}: {
  id: string;
  label: string;
  value: number;
  onChange: (value: number) => void;
}) {
  return (
    <div className="grid gap-2">
      <Label htmlFor={id}>{label}</Label>
      <Input
        id={id}
        type="number"
        min={1}
        value={value}
        onChange={(event) => onChange(Number(event.target.value))}
      />
    </div>
  );
}

function ScopeCheckboxes({
  scope,
  onChange,
}: {
  scope: CreateAccessReviewScheduleInput['scope'];
  onChange: (scope: CreateAccessReviewScheduleInput['scope']) => void;
}) {
  return (
    <div className="grid gap-3">
      <Label>Scope</Label>
      {scopeOptions.map(([key, label]) => (
        <label key={key} className="flex items-center gap-2 text-sm">
          <Checkbox
            checked={scope[key]}
            onCheckedChange={(checked) => onChange({ ...scope, [key]: checked === true })}
          />
          <span>{label}</span>
        </label>
      ))}
    </div>
  );
}

function hasScope(scope: Record<ScopeKey, boolean>) {
  return scopeOptions.some(([key]) => scope[key]);
}
