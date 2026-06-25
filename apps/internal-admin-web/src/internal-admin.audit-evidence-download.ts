import { exportAuditEvidence } from './internal-admin.api';
import type { AdminCredentials, AuditEventFilters } from './internal-admin.types';

export type EvidenceExportState = {
  setError: (value: string | null) => void;
  setIsExporting: (value: boolean) => void;
};

export async function downloadEvidenceExport(
  credentials: AdminCredentials,
  filters: AuditEventFilters,
  state: EvidenceExportState,
) {
  state.setError(null);
  state.setIsExporting(true);
  try {
    const evidence = await exportAuditEvidence(credentials, filters);
    const blob = new Blob([JSON.stringify(evidence, null, 2)], {
      type: 'application/json',
    });
    const url = URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = url;
    link.download = `nvbes-audit-evidence-${evidence.workspace_id}-${evidence.export_id}.json`;
    link.click();
    URL.revokeObjectURL(url);
  } catch (error) {
    state.setError(error instanceof Error ? error.message : 'Evidence export failed');
  } finally {
    state.setIsExporting(false);
  }
}
