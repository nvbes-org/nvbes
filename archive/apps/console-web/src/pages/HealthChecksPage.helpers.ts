import type { DeveloperHealthCheck } from '../developer.schemas';

export function countUnhealthyChecks(checks: DeveloperHealthCheck[]): number {
  return checks.filter((check) => check.status === 'warning' || check.status === 'failing').length;
}

export function healthStatusClass(status: DeveloperHealthCheck['status']): string {
  switch (status) {
    case 'passing':
      return 'text-emerald-700';
    case 'warning':
      return 'text-amber-700';
    case 'failing':
      return 'text-red-700';
    case 'unknown':
      return 'text-muted-foreground';
  }
}
