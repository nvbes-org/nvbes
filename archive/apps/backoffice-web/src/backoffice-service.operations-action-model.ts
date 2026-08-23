import {
  replayExportRun,
  replayOperationsJobRun,
  replayOperationsProviderEvent,
  resolveReconciliationDifference,
  scheduleMaintenanceWindow,
  updateOperationsIncidentState,
} from './backoffice-service.api';
import type { AdminCredentials } from './backoffice-service.types';

export type OperationsActionKind =
  | 'incidentState'
  | 'maintenanceWindow'
  | 'replayExport'
  | 'replayJob'
  | 'replayProvider'
  | 'resolveRecon';

export type IncidentStatus = 'mitigating' | 'open' | 'resolved';

export const operationsConfirmCodes: Record<OperationsActionKind, string> = {
  incidentState: 'UPDATE INCIDENT',
  maintenanceWindow: 'SCHEDULE MAINTENANCE',
  replayJob: 'REPLAY JOB RUN',
  replayProvider: 'REPLAY PROVIDER EVENT',
  replayExport: 'REPLAY EXPORT RUN',
  resolveRecon: 'RESOLVE RECON DIFFERENCE',
};

export type OperationsActionPayload = {
  confirmCode: string;
  incidentStatus: IncidentStatus;
  maintenanceTitle: string;
  reason: string;
  scheduledEnd: string;
  scheduledStart: string;
  targetId: string;
};

export function executeOperationsAction(
  credentials: AdminCredentials,
  action: OperationsActionKind,
  payload: OperationsActionPayload,
) {
  const body = {
    confirm_code: payload.confirmCode,
    reason: payload.reason,
  };
  if (action === 'incidentState') {
    return updateOperationsIncidentState(credentials, payload.targetId, {
      ...body,
      status: payload.incidentStatus,
    });
  }
  if (action === 'maintenanceWindow') {
    return scheduleMaintenanceWindow(credentials, {
      ...body,
      scheduled_end_at: payload.scheduledEnd,
      scheduled_start_at: payload.scheduledStart,
      title: payload.maintenanceTitle,
    });
  }
  if (action === 'replayJob') return replayOperationsJobRun(credentials, payload.targetId, body);
  if (action === 'replayProvider') {
    return replayOperationsProviderEvent(credentials, payload.targetId, body);
  }
  if (action === 'replayExport') return replayExportRun(credentials, payload.targetId, body);
  return resolveReconciliationDifference(credentials, payload.targetId, body);
}

export function targetLabel(action: OperationsActionKind): string {
  if (action === 'incidentState') return 'Incident ID';
  if (action === 'replayJob') return 'Job run ID';
  if (action === 'replayProvider') return 'Provider event ID';
  if (action === 'replayExport') return 'Export run ID';
  return 'Reconciliation difference ID';
}

export function parseIncidentStatus(value: string): IncidentStatus {
  if (value === 'open' || value === 'resolved') return value;
  return 'mitigating';
}
