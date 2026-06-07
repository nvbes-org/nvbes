import { replayQueue } from './drive.sw.queue';
import { setDriveServiceWorkerSentryConsent } from './drive.sw.sentry';
import { driveWorkerSelf, type ExtendableMessageEventLike } from './drive.sw.events.shared';

type DriveServiceWorkerMessage = {
  id?: string;
  isSlowConnection?: boolean;
  sentryAccepted?: boolean;
  tag?: string;
  type?: string;
};

function messageDataOf(event: ExtendableMessageEventLike): DriveServiceWorkerMessage {
  return typeof event.data === 'object' && event.data !== null
    ? (event.data as DriveServiceWorkerMessage)
    : {};
}

export function registerDriveServiceWorkerMessageEvent({
  getIsSlowConnection,
  setIsSlowConnection,
}: {
  getIsSlowConnection: () => boolean;
  setIsSlowConnection: (value: boolean) => void;
}) {
  driveWorkerSelf.addEventListener('message', (event: ExtendableMessageEventLike) => {
    const data = messageDataOf(event);
    const { id, type } = data;

    if (type === 'SKIP_WAITING') {
      void driveWorkerSelf.skipWaiting();
      return;
    }

    if (type === 'NETWORK_QUALITY') {
      setIsSlowConnection(data.isSlowConnection === true);
      return;
    }

    if (type === 'SENTRY_CONSENT_UPDATED') {
      setDriveServiceWorkerSentryConsent(data.sentryAccepted === true);
      return;
    }

    const port = event.ports?.[0];
    if (!port) {
      return;
    }

    switch (type) {
      case 'PING': {
        port.postMessage({ id, result: 'pong' });
        return;
      }
      case 'SYNC_NOW': {
        if (getIsSlowConnection()) {
          port.postMessage({ id, error: 'Sync deferred: slow connection' });
          return;
        }

        if (!data.tag) {
          port.postMessage({ id, error: 'Missing tag' });
          return;
        }

        event.waitUntil(
          replayQueue(data.tag).finally(() => port.postMessage({ id, result: 'ok' })),
        );
        return;
      }
      case 'REGISTER_PERIODIC_SYNC': {
        port.postMessage({ id, result: 'ok' });
        return;
      }
      default:
        port.postMessage({ id, error: `Unknown message type: ${type}` });
    }
  });
}
