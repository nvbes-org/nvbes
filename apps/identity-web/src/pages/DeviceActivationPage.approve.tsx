import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import type { DeviceActivationApproveProps } from '@/pages/DeviceActivationPage.types';

export function DeviceActivationApproveStep({
  deviceInfo,
  workspaces,
  selectedWorkspace,
  lockedWorkspace,
  loading,
  onWorkspaceChange,
  onApprove,
  onDeny,
}: DeviceActivationApproveProps) {
  return (
    <div className="space-y-6">
      <Card className="space-y-2 p-4">
        <p className="text-sm font-medium">
          Application: <span className="font-bold">{deviceInfo.client_name}</span>
        </p>
        <p className="text-xs text-muted-foreground">
          Demande l&apos;accès aux permissions suivantes:
        </p>
        <ul className="list-inside list-disc space-y-1 text-xs">
          {deviceInfo.scope.map((scope) => (
            <li key={scope}>{scope}</li>
          ))}
        </ul>
      </Card>

      <div className="space-y-2">
        <Label htmlFor="workspace-select">
          Workspace courant
        </Label>
        <Select
          value={selectedWorkspace}
          onValueChange={onWorkspaceChange}
          disabled={lockedWorkspace}
        >
          <SelectTrigger id="workspace-select" className="w-full">
            <SelectValue placeholder="Selectionner un workspace" />
          </SelectTrigger>
          <SelectContent>
            <SelectGroup>
              {workspaces.map((workspace) => (
                <SelectItem key={workspace.id} value={workspace.id}>
                  {workspace.name}
                </SelectItem>
              ))}
            </SelectGroup>
          </SelectContent>
        </Select>
        {lockedWorkspace ? (
          <p className="text-xs text-muted-foreground">
            Le workspace courant est utilisé automatiquement.
          </p>
        ) : null}
      </div>

      <div className="grid grid-cols-2 gap-4">
        <Button variant="outline" onClick={onDeny} disabled={loading} className="h-12">
          Refuser
        </Button>
        <Button onClick={onApprove} disabled={loading || !selectedWorkspace} className="h-12">
          Approuver
        </Button>
      </div>
    </div>
  );
}
