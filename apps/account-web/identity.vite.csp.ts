import type { Plugin } from 'vite-plus';

export const permissionsPolicy =
  'accelerometer=(), camera=(), geolocation=(), gyroscope=(), magnetometer=(), microphone=(), payment=(), usb=()';
export const integrityPolicyStyles = 'blocked-destinations=(style)';

export function getCsp(
  mode: string,
  sentryConnectUrl: string,
  posthogConnectUrl: string,
  faroConnectUrl: string,
): string {
  const isDev = mode === 'development';

  const sentryConnect = sentryConnectUrl ? ` ${sentryConnectUrl}` : '';
  const posthogConnect = posthogConnectUrl ? ` ${posthogConnectUrl}` : '';
  const faroConnect = faroConnectUrl ? ` ${faroConnectUrl}` : '';

  if (isDev) {
    return `default-src 'self'; script-src 'self' 'unsafe-inline' 'unsafe-eval'; worker-src 'self' blob:; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; img-src 'self' data: blob: https:; font-src 'self' data: https://fonts.gstatic.com; connect-src 'self' ws://localhost:* http://localhost:*${sentryConnect}${posthogConnect}${faroConnect} http://localhost:8080; frame-src 'self'; object-src 'none'; base-uri 'self'; form-action 'self'; frame-ancestors 'none'; report-uri /csp-report; upgrade-insecure-requests;`;
  }
  return `default-src 'self'; script-src 'self'; worker-src 'self' blob:; style-src 'self' https://fonts.googleapis.com; img-src 'self' data: blob: https:; font-src 'self' data: https://fonts.gstatic.com; connect-src 'self'${sentryConnect}${posthogConnect}${faroConnect}; frame-src 'self'; object-src 'none'; base-uri 'self'; form-action 'self'; frame-ancestors 'none'; report-uri /csp-report; upgrade-insecure-requests;`;
}

export function originFromUrl(value: string): string {
  try {
    return value ? new URL(value).origin : '';
  } catch {
    return '';
  }
}

export function cspPlugin(
  mode: string,
  sentryConnectUrl: string,
  posthogConnectUrl: string,
  faroConnectUrl: string,
): Plugin {
  return {
    name: 'csp-injection-plugin',
    transformIndexHtml(html: string) {
      const cspString = getMetaCsp(mode, sentryConnectUrl, posthogConnectUrl, faroConnectUrl);
      const metaTag = `<meta http-equiv="Content-Security-Policy" content="${cspString}" />`;
      return html.replace('<!-- %CSP_META% -->', metaTag);
    },
  };
}

function getMetaCsp(
  mode: string,
  sentryConnectUrl: string,
  posthogConnectUrl: string,
  faroConnectUrl: string,
): string {
  return getCsp(mode, sentryConnectUrl, posthogConnectUrl, faroConnectUrl)
    .replace("; frame-ancestors 'none'", '')
    .replace('; report-uri /csp-report', '');
}
