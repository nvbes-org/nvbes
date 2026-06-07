import { registerDriveServiceWorkerBackgroundFetchEvents } from './drive.sw.events.background-fetch';
import { registerDriveServiceWorkerMessageEvent } from './drive.sw.events.message';
import { driveWorkerSelf, type PeriodicSyncEvent } from './drive.sw.events.shared';
import { replayQueue } from './drive.sw.queue';
import { captureDriveServiceWorkerException } from './drive.sw.sentry';

export function registerDriveServiceWorkerEvents({
  getIsSlowConnection,
  setIsSlowConnection,
}: {
  getIsSlowConnection: () => boolean;
  setIsSlowConnection: (value: boolean) => void;
}) {
  driveWorkerSelf.addEventListener('error', (event: ErrorEvent) => {
    captureDriveServiceWorkerException(event.error ?? event.message, 'global.error', {
      filename: event.filename,
      lineno: event.lineno,
      colno: event.colno,
    });
  });

  driveWorkerSelf.addEventListener('unhandledrejection', (event: PromiseRejectionEvent) => {
    captureDriveServiceWorkerException(event.reason, 'global.unhandledrejection');
  });

  registerDriveServiceWorkerMessageEvent({
    getIsSlowConnection,
    setIsSlowConnection,
  });

  driveWorkerSelf.addEventListener('periodicsync', (event: PeriodicSyncEvent) => {
    const syncEvent = event as PeriodicSyncEvent;
    syncEvent.waitUntil(replayQueue(syncEvent.tag));
  });

  registerDriveServiceWorkerBackgroundFetchEvents();
}
