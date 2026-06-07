import { Button } from '@/components/ui/button';
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
      <div className="space-y-2 rounded-lg bg-muted p-4">
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
      </div>

      <div className="space-y-2">
        <label htmlFor="workspace-select" className="text-sm font-medium">
          Workspace courant
        </label>
        <select
          id="workspace-select"
          value={selectedWorkspace}
          onChange={(event) => onWorkspaceChange(event.target.value)}
          disabled={lockedWorkspace}
          className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
        >
          {workspaces.map((workspace) => (
            <option key={workspace.id} value={workspace.id}>
              {workspace.name}
            </option>
          ))}
        </select>
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
