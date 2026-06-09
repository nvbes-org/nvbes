import { Shield } from 'lucide-react';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Spinner } from '@/components/ui/spinner';
import { Textarea } from '@/components/ui/textarea';
import type { ServiceAccount } from '../identity.service-accounts.api';
import { badgeVariantForStatus, statusLabel } from './WorkspaceServiceAccounts.helpers';

export function ServiceAccountConfiguration({
  selectedServiceAccount,
  updateName,
  setUpdateName,
  updateDescription,
  setUpdateDescription,
  updateRole,
  setUpdateRole,
  busyAction,
  editError,
  onSave,
}: {
  selectedServiceAccount: ServiceAccount;
  updateName: string;
  setUpdateName: (value: string) => void;
  updateDescription: string;
  setUpdateDescription: (value: string) => void;
  updateRole: string;
  setUpdateRole: (value: string) => void;
  busyAction: string | null;
  editError: string | null;
  onSave: () => void;
}) {
  return (
    <Card className="p-5">
      <div className="flex items-center justify-between gap-3">
        <div>
          <p className="text-sm font-medium">Configuration</p>
          <p className="text-sm text-muted-foreground">
            Mettre a jour le nom, la description et le role RBAC.
          </p>
        </div>
        <Badge variant={badgeVariantForStatus(selectedServiceAccount.status)}>
          {statusLabel(selectedServiceAccount.status)}
        </Badge>
      </div>

      <div className="mt-4 grid gap-4">
        <div className="grid gap-2">
          <Label htmlFor="service-account-name">
            Nom
          </Label>
          <Input
            id="service-account-name"
            value={updateName}
            onChange={(event) => setUpdateName(event.target.value)}
          />
        </div>

        <div className="grid gap-2">
          <Label htmlFor="service-account-description">
            Description
          </Label>
          <Textarea
            id="service-account-description"
            className="min-h-24"
            value={updateDescription}
            onChange={(event) => setUpdateDescription(event.target.value)}
          />
        </div>

        <div className="grid gap-2">
          <Label htmlFor="service-account-role">
            Role RBAC
          </Label>
          <Select
            value={updateRole}
            onValueChange={setUpdateRole}
          >
            <SelectTrigger id="service-account-role" className="w-full">
              <SelectValue placeholder="Choisir un role" />
            </SelectTrigger>
            <SelectContent>
              <SelectGroup>
                <SelectItem value="viewer">viewer</SelectItem>
                <SelectItem value="member">member</SelectItem>
                <SelectItem value="admin">admin</SelectItem>
                <SelectItem value="owner">owner</SelectItem>
              </SelectGroup>
            </SelectContent>
          </Select>
        </div>

        {editError ? (
          <Alert variant="destructive">
            <AlertDescription>{editError}</AlertDescription>
          </Alert>
        ) : null}
      </div>

      <div className="mt-4 flex flex-wrap gap-2">
        <Button onClick={onSave} disabled={busyAction === 'update-service-account'}>
          {busyAction === 'update-service-account' ? (
            <>
              <Spinner />
              Sauvegarde...
            </>
          ) : (
            <>
              <Shield data-icon="inline-start" />
              Sauvegarder
            </>
          )}
        </Button>
      </div>
    </Card>
  );
}
