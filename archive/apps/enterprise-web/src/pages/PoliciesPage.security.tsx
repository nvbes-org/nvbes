import type { EnterprisePoliciesResponse } from '@nvbes/identity-client';
import { Clock3, Save, ShieldCheck } from 'lucide-react';
import { useEffect, useState } from 'react';
import { Badge } from '../components/ui/badge';
import { Button } from '../components/ui/button';
import { Card, CardContent } from '../components/ui/card';
import { Input } from '../components/ui/input';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '../components/ui/select';

type MfaPolicyValue = EnterprisePoliciesResponse['mfa_policy']['policy'];

type SessionPolicyCardProps = {
  error: Error | null;
  pending: boolean;
  onSave(value: number): Promise<void>;
  sessionPolicy: EnterprisePoliciesResponse['session_policy'];
};

export function SessionPolicyCard(props: SessionPolicyCardProps) {
  const { error, pending, sessionPolicy } = props;
  const [value, setValue] = useState(String(sessionPolicy.admin_session_ttl_hours));
  const parsedValue = Number.parseInt(value, 10);
  const validValue = Number.isInteger(parsedValue) && parsedValue >= 1 && parsedValue <= 168;
  const dirty = parsedValue !== sessionPolicy.admin_session_ttl_hours;

  useEffect(() => {
    setValue(String(sessionPolicy.admin_session_ttl_hours));
  }, [sessionPolicy.admin_session_ttl_hours]);

  return (
    <Card className="rounded-lg" size="sm">
      <CardContent className="flex flex-col gap-4 md:flex-row md:items-start md:justify-between">
        <div className="flex min-w-0 items-start gap-3">
          <div className="rounded-lg border border-border bg-muted/30 p-2 text-muted-foreground">
            <Clock3 className="size-4" />
          </div>
          <div className="min-w-0">
            <PolicyHeading compliant={sessionPolicy.compliant} title="Admin session TTL" />
            <p className="mt-1 text-sm leading-5 text-muted-foreground">
              Current admin session TTL is {sessionPolicy.admin_session_ttl_hours}h. Recommended
              maximum is {sessionPolicy.recommended_admin_session_ttl_hours}h with step-up for admin
              elevation. Source: {sessionPolicy.source}.
            </p>
            {error ? <p className="mt-2 text-sm text-destructive">{error.message}</p> : null}
          </div>
        </div>
        <form
          className="grid gap-2 md:min-w-64"
          onSubmit={(event) => {
            event.preventDefault();
            if (validValue) {
              void props.onSave(parsedValue);
            }
          }}
        >
          <label className="text-xs font-medium text-muted-foreground" htmlFor="admin-session-ttl">
            Admin session TTL hours
          </label>
          <div className="flex gap-2">
            <Input
              id="admin-session-ttl"
              min={1}
              max={168}
              type="number"
              value={value}
              onChange={(event) => setValue(event.target.value)}
            />
            <Button type="submit" disabled={!dirty || !validValue || pending}>
              <Save className="size-4" />
              Save
            </Button>
          </div>
          <p className="text-xs text-muted-foreground">Allowed range: 1-168 hours.</p>
        </form>
      </CardContent>
    </Card>
  );
}

type MfaPolicyCardProps = {
  adminWithoutMfaCount: number;
  error: Error | null;
  mfaPolicy: EnterprisePoliciesResponse['mfa_policy'];
  pending: boolean;
  onSave(value: MfaPolicyValue): Promise<void>;
};

export function MfaPolicyCard(props: MfaPolicyCardProps) {
  const { adminWithoutMfaCount, error, mfaPolicy, pending } = props;
  const [value, setValue] = useState<MfaPolicyValue>(mfaPolicy.policy);
  const dirty = value !== mfaPolicy.policy;

  useEffect(() => {
    setValue(mfaPolicy.policy);
  }, [mfaPolicy.policy]);

  return (
    <Card className="rounded-lg" size="sm">
      <CardContent className="flex flex-col gap-4 md:flex-row md:items-start md:justify-between">
        <div className="flex min-w-0 items-start gap-3">
          <div className="rounded-lg border border-border bg-muted/30 p-2 text-muted-foreground">
            <ShieldCheck className="size-4" />
          </div>
          <div className="min-w-0">
            <PolicyHeading compliant={mfaPolicy.compliant} title="MFA enforcement" />
            <p className="mt-1 text-sm leading-5 text-muted-foreground">
              Current policy: {policyLabel(mfaPolicy.policy)}. Recommended policy:{' '}
              {policyLabel(mfaPolicy.recommended_policy)}. Admins without MFA:{' '}
              {adminWithoutMfaCount}.
            </p>
            {error ? <p className="mt-2 text-sm text-destructive">{error.message}</p> : null}
          </div>
        </div>
        <form
          className="grid gap-2 md:min-w-72"
          onSubmit={(event) => {
            event.preventDefault();
            void props.onSave(value);
          }}
        >
          <label className="text-xs font-medium text-muted-foreground" htmlFor="mfa-policy">
            Tenant MFA policy
          </label>
          <div className="flex gap-2">
            <Select value={value} onValueChange={(next) => setValue(next as MfaPolicyValue)}>
              <SelectTrigger id="mfa-policy" className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="optional">Optional</SelectItem>
                <SelectItem value="required_admins">Required for admins</SelectItem>
                <SelectItem value="required_all">Required for everyone</SelectItem>
              </SelectContent>
            </Select>
            <Button type="submit" disabled={!dirty || pending}>
              <Save className="size-4" />
              Save
            </Button>
          </div>
          <p className="text-xs text-muted-foreground">
            Admin enforcement covers owners and admins at sign-in.
          </p>
        </form>
      </CardContent>
    </Card>
  );
}

function PolicyHeading({ compliant, title }: { compliant: boolean; title: string }) {
  return (
    <div className="flex flex-wrap items-center gap-2">
      <p className="text-sm font-semibold">{title}</p>
      <Badge variant={compliant ? 'default' : 'secondary'} className="rounded-md">
        {compliant ? 'Compliant' : 'Review'}
      </Badge>
    </div>
  );
}

function policyLabel(policy: MfaPolicyValue): string {
  switch (policy) {
    case 'required_admins':
      return 'required for admins';
    case 'required_all':
      return 'required for everyone';
    default:
      return 'optional';
  }
}
