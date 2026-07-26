import { ShieldCheck } from 'lucide-react';
import type { SubmitEvent } from 'react';
import { Alert, AlertDescription, AlertTitle } from '../components/ui/alert';
import { Button } from '../components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '../components/ui/dialog';
import { Field, FieldLabel } from '../components/ui/field';
import { Input } from '../components/ui/input';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '../components/ui/select';
import { Textarea } from '../components/ui/textarea';

export type StepUpMethod = 'password' | 'webauthn' | 'totp' | 'recovery';

export function AdminElevationDialog({
  open,
  method,
  password,
  totpCode,
  recoveryCode,
  breakGlassActive,
  breakGlassProcedureReference,
  breakGlassReason,
  pending,
  error,
  onOpenChange,
  onMethodChange,
  onPasswordChange,
  onTotpCodeChange,
  onRecoveryCodeChange,
  onBreakGlassReasonChange,
  onSubmit,
}: {
  open: boolean;
  method: StepUpMethod;
  password: string;
  totpCode: string;
  recoveryCode: string;
  breakGlassActive: boolean;
  breakGlassProcedureReference: string | null;
  breakGlassReason: string;
  pending: boolean;
  error: string | null;
  onOpenChange: (open: boolean) => void;
  onMethodChange: (method: StepUpMethod) => void;
  onPasswordChange: (value: string) => void;
  onTotpCodeChange: (value: string) => void;
  onRecoveryCodeChange: (value: string) => void;
  onBreakGlassReasonChange: (value: string) => void;
  onSubmit: (event: SubmitEvent<HTMLFormElement>) => void;
}) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-md">
        <form className="flex flex-col gap-4" onSubmit={onSubmit}>
          <DialogHeader>
            <DialogTitle>Admin verification</DialogTitle>
            <DialogDescription>
              Temporary admin elevation is required before changing sensitive tenant settings.
            </DialogDescription>
          </DialogHeader>

          <Alert>
            <ShieldCheck className="size-4" />
            <AlertTitle>Just-in-time elevation</AlertTitle>
            <AlertDescription>
              The admin window lasts 15 minutes and cannot outlive this session step-up.
            </AlertDescription>
          </Alert>

          <Field>
            <FieldLabel>Verification method</FieldLabel>
            <Select value={method} onValueChange={(value) => onMethodChange(value as StepUpMethod)}>
              <SelectTrigger className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="password">Password</SelectItem>
                <SelectItem value="webauthn">Passkey or security key</SelectItem>
                <SelectItem value="totp">Authenticator code</SelectItem>
                <SelectItem value="recovery">Recovery code</SelectItem>
              </SelectContent>
            </Select>
          </Field>

          <StepUpField
            method={method}
            password={password}
            totpCode={totpCode}
            recoveryCode={recoveryCode}
            onPasswordChange={onPasswordChange}
            onTotpCodeChange={onTotpCodeChange}
            onRecoveryCodeChange={onRecoveryCodeChange}
          />

          {breakGlassActive ? (
            <Field>
              <FieldLabel htmlFor="admin-elevation-break-glass-reason">
                Emergency procedure reason
              </FieldLabel>
              <Textarea
                id="admin-elevation-break-glass-reason"
                value={breakGlassReason}
                onChange={(event) => onBreakGlassReasonChange(event.target.value)}
                placeholder={breakGlassProcedureReference ?? 'Procedure reference'}
                required
              />
            </Field>
          ) : null}

          {error ? <p className="text-sm text-destructive">{error}</p> : null}

          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              disabled={pending}
              onClick={() => onOpenChange(false)}
            >
              Cancel
            </Button>
            <Button type="submit" disabled={pending}>
              {pending ? 'Verifying...' : 'Verify and continue'}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

function StepUpField({
  method,
  password,
  totpCode,
  recoveryCode,
  onPasswordChange,
  onTotpCodeChange,
  onRecoveryCodeChange,
}: {
  method: StepUpMethod;
  password: string;
  totpCode: string;
  recoveryCode: string;
  onPasswordChange: (value: string) => void;
  onTotpCodeChange: (value: string) => void;
  onRecoveryCodeChange: (value: string) => void;
}) {
  if (method === 'webauthn') {
    return (
      <p className="text-sm text-muted-foreground">
        Continue to use the passkey or security key registered on this account.
      </p>
    );
  }

  if (method === 'totp') {
    return (
      <Input
        value={totpCode}
        inputMode="numeric"
        autoComplete="one-time-code"
        maxLength={6}
        placeholder="000000"
        onChange={(event) => onTotpCodeChange(event.target.value)}
        required
      />
    );
  }

  if (method === 'recovery') {
    return (
      <Input
        value={recoveryCode}
        placeholder="XXXX-XXXX-XXXX"
        onChange={(event) => onRecoveryCodeChange(event.target.value)}
        required
      />
    );
  }

  return (
    <Input
      value={password}
      type="password"
      autoComplete="current-password"
      placeholder="Password"
      onChange={(event) => onPasswordChange(event.target.value)}
      required
    />
  );
}
