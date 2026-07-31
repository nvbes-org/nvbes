import type { Plugin } from 'vite-plus';
import {
  browserIsolationHeaders,
  buildWebCsp,
  cspMetaFromHeader,
  integrityPolicyScripts,
  originFromUrl,
  permissionsPolicy,
  posthogAssetsOriginFromHost,
  productionTransportHeaders,
  uaClientHintsHeaders,
} from '../../libs/ts/web-runtime/src/csp';

export function getCsp(
  mode: string,
  sentryConnectUrl: string,
  posthogConnectUrl: string,
  faroConnectUrl: string,
  resourceConnectUrls: readonly string[],
  allowLocalHttpSources = false,
): string {
  const isDev = mode === 'development';
  const localHttpSourcesAllowed = isDev || allowLocalHttpSources;

  const sentryConnect = sentryConnectUrl ? ` ${sentryConnectUrl}` : '';
  const posthogConnect = posthogConnectUrl ? ` ${posthogConnectUrl}` : '';
  const posthogAssetsUrl = posthogAssetsOriginFromHost(posthogConnectUrl);
  const faroConnect = faroConnectUrl ? ` ${faroConnectUrl}` : '';

  return buildWebCsp({
    mode,
    scriptSrc: [posthogAssetsUrl],
    styleSrc: ['https://fonts.googleapis.com'],
    imgSrc: [
      'https:',
      ...(localHttpSourcesAllowed ? ['http://localhost:*', 'http://127.0.0.1:*'] : []),
    ],
    fontSrc: ['https://fonts.gstatic.com'],
    connectSrc: [
      sentryConnect.trim(),
      posthogConnect.trim(),
      posthogAssetsUrl,
      faroConnect.trim(),
      ...resourceConnectUrls,
      ...(isDev ? ['http://localhost:8080', 'http://localhost:*', 'http://127.0.0.1:*'] : []),
    ],
  });
}

export function cspPlugin(
  mode: string,
  sentryConnectUrl: string,
  posthogConnectUrl: string,
  faroConnectUrl: string,
  resourceConnectUrls: readonly string[],
  allowLocalHttpSources = false,
): Plugin {
  return {
    name: 'csp-injection-plugin',
    transformIndexHtml(html: string) {
      const cspString = getMetaCsp(
        mode,
        sentryConnectUrl,
        posthogConnectUrl,
        faroConnectUrl,
        resourceConnectUrls,
        allowLocalHttpSources,
      );
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
  resourceConnectUrls: readonly string[],
  allowLocalHttpSources: boolean,
): string {
  return cspMetaFromHeader(
    getCsp(
      mode,
      sentryConnectUrl,
      posthogConnectUrl,
      faroConnectUrl,
      resourceConnectUrls,
      allowLocalHttpSources,
    ),
  );
}

export {
  browserIsolationHeaders,
  integrityPolicyScripts,
  originFromUrl,
  permissionsPolicy,
  productionTransportHeaders,
  uaClientHintsHeaders,
};
