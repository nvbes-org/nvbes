type BadgeVariant = 'default' | 'secondary' | 'destructive' | 'outline' | 'ghost' | 'link';

export const driveScopesPreset = [
  'drive.files.read',
  'drive.files.write',
  'drive.files.delete',
  'drive.share_links.read',
  'drive.share_links.write',
  'drive.workspace.read',
].join(', ');

export type SecretResult =
  | {
      kind: 'created';
      client_id: string;
      client_secret: string;
      timestamp: string;
    }
  | {
      kind: 'rotated';
      client_id: string;
      client_secret: string;
      timestamp: string;
    };

export function formatDateTime(value: string): string {
  return new Intl.DateTimeFormat('fr-FR', {
    dateStyle: 'medium',
    timeStyle: 'short',
  }).format(new Date(value));
}

export function splitList(value: string): string[] {
  return value
    .split(/[\n,]/)
    .map((part) => part.trim())
    .filter(Boolean);
}

export function badgeVariantForStatus(status: string): BadgeVariant {
  if (status === 'active') return 'default';
  if (status === 'suspended') return 'destructive';
  return 'secondary';
}

export function statusLabel(status: string): string {
  if (status === 'active') return 'Actif';
  if (status === 'suspended') return 'Suspendu';
  return status;
}
