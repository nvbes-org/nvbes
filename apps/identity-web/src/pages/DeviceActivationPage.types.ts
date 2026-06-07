import type { AccountWorkspace } from '@nvbes/identity-client';

export interface DeviceInfo {
  client_name: string;
  scope: string[];
  tenant_id: string;
}

export type DeviceActivationStep = 'input' | 'approve' | 'success' | 'error';

export type DeviceActivationApproveProps = {
  deviceInfo: DeviceInfo;
  workspaces: AccountWorkspace[];
  selectedWorkspace: string;
  lockedWorkspace: boolean;
  loading: boolean;
  onWorkspaceChange: (value: string) => void;
  onApprove: () => void;
  onDeny: () => void;
};
