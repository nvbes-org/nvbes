import { getSafeLocalStorage } from '@nvbes/web-runtime';

const storageKey = 'nvbes.internal-admin.audit-views';
export const allAuditFilterValue = '__all__';

export type AuditViewFilters = {
  action: string;
  query: string;
  sort: AuditSortMode;
  targetType: string;
};

export type AuditSortMode = 'hash_anomalies' | 'newest' | 'oldest' | 'sensitive_first';

export type SavedAuditView = AuditViewFilters & {
  id: string;
  name: string;
  updatedAt: string;
};

export const emptyAuditFilters: AuditViewFilters = {
  action: allAuditFilterValue,
  query: '',
  sort: 'newest',
  targetType: allAuditFilterValue,
};

export function loadSavedAuditViews(): SavedAuditView[] {
  try {
    const raw = getSafeLocalStorage().getItem(storageKey);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as unknown;
    if (!Array.isArray(parsed)) return [];
    return parsed.flatMap((item) => (isSavedAuditView(item) ? [item] : []));
  } catch {
    return [];
  }
}

export function saveAuditViews(views: SavedAuditView[]) {
  getSafeLocalStorage().setItem(storageKey, JSON.stringify(views));
}

export function upsertAuditView(
  views: SavedAuditView[],
  name: string,
  filters: AuditViewFilters,
): SavedAuditView[] {
  const normalizedName = name.trim();
  if (!normalizedName) return views;
  const existing = views.find((view) => view.name.toLowerCase() === normalizedName.toLowerCase());
  const nextView: SavedAuditView = {
    ...filters,
    id: existing?.id ?? crypto.randomUUID(),
    name: normalizedName,
    updatedAt: new Date().toISOString(),
  };
  return [nextView, ...views.filter((view) => view.id !== nextView.id)].slice(0, 12);
}

export function removeAuditView(views: SavedAuditView[], viewId: string): SavedAuditView[] {
  return views.filter((view) => view.id !== viewId);
}

function isSavedAuditView(value: unknown): value is SavedAuditView {
  if (!isRecord(value)) return false;
  return (
    typeof value.id === 'string' &&
    typeof value.name === 'string' &&
    typeof value.updatedAt === 'string' &&
    typeof value.action === 'string' &&
    typeof value.query === 'string' &&
    isAuditSortMode(value.sort) &&
    typeof value.targetType === 'string'
  );
}

function isAuditSortMode(value: unknown): value is AuditSortMode {
  return (
    value === 'hash_anomalies' ||
    value === 'newest' ||
    value === 'oldest' ||
    value === 'sensitive_first'
  );
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}
