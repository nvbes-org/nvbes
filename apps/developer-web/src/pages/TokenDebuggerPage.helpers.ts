import type { DebugDeveloperToken } from '../developer.schemas';

export function summarizeAccessDecision(result: DebugDeveloperToken | null): string {
  if (!result) {
    return 'No token inspected';
  }

  switch (result.access_decision) {
    case 'allowed':
      return 'Access allowed';
    case 'expired':
      return 'Token expired';
    case 'tenant_mismatch':
      return 'Tenant mismatch';
    case 'invalid':
      return 'Invalid token';
  }
}
