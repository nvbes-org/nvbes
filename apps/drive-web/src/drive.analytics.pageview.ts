import { trackProductEvent } from '@nvbes/web-runtime/analytics';

const DRIVE_SENSITIVE_ROUTES = [
  /^\/callback(?:\/|$)/u,
  /\/(?:files?|folders?|previews?|uploads?|downloads?|share-links?)(?:\/|$)/u,
  /\/(?:billing|checkout|portal)(?:\/|$)/u,
  /\/(?:privacy|export|delete|tokens?|service-accounts?)(?:\/|$)/u,
] as const;

let pageTrackingInstalled = false;
let lastTrackedPath: string | null = null;

export function currentPath(): string {
  if (typeof window === 'undefined') {
    return '/';
  }
  return window.location.pathname || '/';
}

export function captureCurrentPageView(): void {
  const routePath = currentPath();
  if (routePath === lastTrackedPath) {
    return;
  }

  lastTrackedPath = routePath;
  void trackProductEvent('marketing.page_viewed', {
    event_source: 'router',
    source: 'drive-web',
  });
}

function schedulePageView(): void {
  window.requestAnimationFrame(captureCurrentPageView);
}

export function installPageTracking(): void {
  if (pageTrackingInstalled || typeof window === 'undefined') {
    return;
  }

  const originalPushState: History['pushState'] = window.history.pushState.bind(window.history);
  const originalReplaceState: History['replaceState'] = window.history.replaceState.bind(
    window.history,
  );

  window.history.pushState = function pushStateWithAnalyticsTracking(
    ...args: Parameters<History['pushState']>
  ) {
    const result = originalPushState(...args);
    schedulePageView();
    return result;
  };

  window.history.replaceState = function replaceStateWithAnalyticsTracking(
    ...args: Parameters<History['replaceState']>
  ) {
    const result = originalReplaceState(...args);
    schedulePageView();
    return result;
  };

  window.addEventListener('popstate', schedulePageView);
  pageTrackingInstalled = true;
}

export const blockedRoutePatterns = [...DRIVE_SENSITIVE_ROUTES];
