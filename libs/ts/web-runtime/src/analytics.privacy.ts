import type { AnalyticsProperties, AnalyticsPurposeConsent } from './analytics.types';
import { getSafeLocalStorage, getSafeSessionStorage } from './safe-storage';

export type PseudonymizeAnalyticsId = (prefix: string, id: string) => Promise<string>;

export const DEFAULT_BLOCKED_ROUTE_PATTERNS = [
  /^\/login(?:\/|$)/u,
  /^\/register(?:\/|$)/u,
  /^\/forgot-password(?:\/|$)/u,
  /^\/reset-password(?:\/|$)/u,
  /^\/verify(?:\/|$)/u,
  /^\/verify-email(?:\/|$)/u,
  /^\/verify-result(?:\/|$)/u,
  /^\/activate(?:\/|$)/u,
  /^\/account\/privacy(?:\/|$)/u,
  /^\/account\/mfa(?:\/|$)/u,
  /^\/account\/security(?:\/|$)/u,
  /^\/account\/billing(?:\/|$)/u,
  /^\/account\/subscriptions(?:\/|$)/u,
  /^\/callback(?:\/|$)/u,
] as const;

export const ALLOWED_EVENTS = new Set([
  'marketing.page_viewed',
  'marketing.cta_clicked',
  'marketing.pricing_viewed',
  'auth.signup_started',
  'auth.signup_completed',
  'auth.email_verified',
  'auth.login_completed',
  'auth.logout_completed',
  'auth.session_revoked',
  'workspace.created',
  'file.upload_started',
  'file.upload_completed',
  'activation.first_file_uploaded',
  'share_link.created',
  'activation.first_share_link_created',
  'member.invited',
  'activation.first_member_invited',
  'activation.workspace_activated',
  'billing.checkout_started',
  'billing.subscription_activated',
  'billing.trial_ending',
  'billing.payment_failed',
  'retention.workspace_active_weekly',
  'retention.workspace_retained_30d',
  'analytics.experiment_exposure',
  'analytics.runtime_error',
]);

const ALLOWED_PROPERTY_KEYS = new Set([
  'app_name',
  'route_path',
  'event_source',
  'source',
  'campaign',
  'country',
  'role',
  'plan_code',
  'workspace_type',
  'workspace_id',
  'user_id',
  'group_id',
  'feature_flag',
  'variant',
  'enabled',
  'status',
  'error_kind',
  'error_name',
  'error_message',
  'duration_ms',
  'file_count',
  'total_bytes_bucket',
  'size_bytes_bucket',
  'upload_count',
  'share_link_count',
  'member_count',
]);

const DENIED_KEY_PATTERN =
  /(email|name|filename|file_name|object_key|token|secret|password|authorization|cookie|checksum|url|href|path)/iu;
const EMAIL_VALUE_PATTERN = /[^\s@]+@[^\s@]+\.[^\s@]+/u;
const JWT_VALUE_PATTERN = /^[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+$/u;
const JWT_IN_TEXT_PATTERN = /[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+/u;
const UUID_VALUE_PATTERN = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/iu;

export function hasAnyAnalyticsConsent(consent: AnalyticsPurposeConsent): boolean {
  return (
    consent.productAnalytics ||
    consent.autocaptureHeatmaps ||
    consent.sessionReplay ||
    consent.surveysFeedback ||
    consent.errorTracking ||
    consent.featureFlags
  );
}

export async function sanitizeAnalyticsProperties(
  input: Record<string, unknown>,
  pseudonymize: PseudonymizeAnalyticsId,
): Promise<AnalyticsProperties> {
  const sanitized: AnalyticsProperties = {};
  for (const [key, value] of Object.entries(input)) {
    const deniedKey = key !== 'route_path' && DENIED_KEY_PATTERN.test(key);
    if (!ALLOWED_PROPERTY_KEYS.has(key) || deniedKey) {
      continue;
    }

    if (key === 'workspace_id' || key === 'user_id' || key === 'group_id') {
      if (typeof value === 'string' && isUuidLike(value)) {
        sanitized[key] = await pseudonymize(key === 'user_id' ? 'usr' : 'wks', value);
      }
      continue;
    }

    const safeValue = safePropertyValue(value);
    if (safeValue !== undefined) {
      sanitized[key] = safeValue;
    }
  }
  return sanitized;
}

export function normalizeAnalyticsError(error: unknown): { name: string; message: string } {
  if (error instanceof Error) {
    return { name: error.name, message: scrubErrorMessage(error.message) };
  }
  return { name: 'UnknownError', message: 'Unknown browser error' };
}

export function clearAnalyticsStorage(): void {
  if (typeof window === 'undefined') {
    return;
  }

  for (const storage of [getSafeLocalStorage(), getSafeSessionStorage()]) {
    for (const key of storage.keys()) {
      const normalized = key.toLowerCase();
      if (normalized.startsWith('ph_') || normalized.startsWith('nvbes.analytics.')) {
        storage.removeItem(key);
      }
    }
  }

  for (const rawCookie of document.cookie.split(';')) {
    const cookieName = rawCookie.split('=')[0]?.trim();
    if (!cookieName) continue;
    const normalized = cookieName.toLowerCase();
    if (!normalized.startsWith('ph_') && !normalized.startsWith('nvbes_analytics_')) continue;

    document.cookie = `${cookieName}=; Max-Age=0; path=/; SameSite=Lax`;
    for (const domain of currentHostCookieDomains()) {
      document.cookie = `${cookieName}=; Max-Age=0; path=/; domain=${domain}; SameSite=Lax`;
    }
  }
}

export function isUuidLike(value: string): boolean {
  return UUID_VALUE_PATTERN.test(value);
}

function safePropertyValue(value: unknown): string | number | boolean | null | undefined {
  if (value === null) return null;
  if (typeof value === 'boolean') return value;
  if (typeof value === 'number') return Number.isFinite(value) ? value : undefined;
  if (typeof value !== 'string') return undefined;
  if (value.length > 200) return undefined;
  if (EMAIL_VALUE_PATTERN.test(value) || JWT_VALUE_PATTERN.test(value)) return undefined;
  if (isUuidLike(value)) return undefined;
  return value;
}

function scrubErrorMessage(message: string): string {
  if (EMAIL_VALUE_PATTERN.test(message) || JWT_IN_TEXT_PATTERN.test(message)) {
    return 'Redacted error message';
  }
  return message.slice(0, 160);
}

function currentHostCookieDomains(): string[] {
  if (typeof window === 'undefined') {
    return [];
  }

  const host = window.location.hostname;
  const parts = host.split('.').filter(Boolean);
  if (parts.length < 2 || host === 'localhost' || /^[\d.]+$/u.test(host)) {
    return [host];
  }

  return [host, `.${parts.slice(-2).join('.')}`];
}
