import type { Plugin } from 'vite-plus';
import {
  buildWebCsp,
  cspMetaFromHeader,
  integrityPolicyScripts,
  originFromUrl,
  permissionsPolicy,
  uaClientHintsHeaders,
} from '../../libs/ts/web-runtime/src/csp';

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

  return buildWebCsp({
    mode,
    styleSrc: ['https://fonts.googleapis.com'],
    imgSrc: ['https:', ...(isDev ? ['http://localhost:*', 'http://127.0.0.1:*'] : [])],
    fontSrc: ['https://fonts.gstatic.com'],
    connectSrc: [
      sentryConnect.trim(),
      posthogConnect.trim(),
      faroConnect.trim(),
      ...(isDev ? ['http://localhost:8080', 'http://localhost:*', 'http://127.0.0.1:*'] : []),
    ],
  });
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
  return cspMetaFromHeader(getCsp(mode, sentryConnectUrl, posthogConnectUrl, faroConnectUrl));
}

export { integrityPolicyScripts, originFromUrl, permissionsPolicy, uaClientHintsHeaders };
