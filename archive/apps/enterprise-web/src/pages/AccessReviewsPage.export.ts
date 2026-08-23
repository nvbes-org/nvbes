import type { AccessReviewCampaignExport } from '@nvbes/identity-client';

export function downloadAccessReviewExport(
  exportData: AccessReviewCampaignExport,
  format: 'csv' | 'json',
) {
  const filename = exportFilename(exportData, format);
  const content =
    format === 'json' ? JSON.stringify(exportData, null, 2) : accessReviewExportToCsv(exportData);
  const type = format === 'json' ? 'application/json' : 'text/csv';
  downloadTextFile(filename, content, type);
}

function accessReviewExportToCsv(exportData: AccessReviewCampaignExport) {
  const rows = [
    [
      'campaign_id',
      'campaign_name',
      'campaign_status',
      'generated_at',
      'item_id',
      'item_type',
      'subject_id',
      'subject_label',
      'workspace_id',
      'role',
      'status',
      'decision',
      'reviewed_by',
      'reviewed_at',
      'created_at',
      'evidence',
    ],
    ...exportData.rows.map((row) => [
      exportData.campaign.id,
      exportData.campaign.name,
      exportData.campaign.status,
      exportData.generated_at,
      row.item_id,
      row.item_type,
      row.subject_id,
      row.subject_label,
      row.workspace_id ?? '',
      row.role ?? '',
      row.status,
      row.decision,
      row.reviewed_by ?? '',
      row.reviewed_at ?? '',
      row.created_at,
      JSON.stringify(row.evidence),
    ]),
  ];
  return rows.map((row) => row.map(escapeCsvCell).join(',')).join('\n');
}

function escapeCsvCell(value: string) {
  if (!/[",\n\r]/u.test(value)) {
    return value;
  }
  return `"${value.replaceAll('"', '""')}"`;
}

function exportFilename(exportData: AccessReviewCampaignExport, format: 'csv' | 'json') {
  const safeName = exportData.campaign.name
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/gu, '-')
    .replace(/^-|-$/gu, '');
  return `access-review-${safeName || exportData.campaign.id}.${format}`;
}

function downloadTextFile(filename: string, content: string, type: string) {
  const blob = new Blob([content], { type });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement('a');
  anchor.href = url;
  anchor.download = filename;
  anchor.click();
  URL.revokeObjectURL(url);
}
