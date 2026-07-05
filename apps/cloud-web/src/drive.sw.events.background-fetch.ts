import {
  BackgroundFetchEvent,
  driveWorkerSelf,
  notifyDriveClients,
} from './drive.sw.events.shared';

export function registerDriveServiceWorkerBackgroundFetchEvents() {
  driveWorkerSelf.addEventListener('backgroundfetchsuccess', (event: BackgroundFetchEvent) => {
    const fetchEvent = event as BackgroundFetchEvent;
    fetchEvent.waitUntil(
      (async () => {
        const records = await fetchEvent.registration.matchAll();
        const results = [];
        for (const record of records) {
          results.push({
            url: record.request.url,
            size: record.responseReady.then((response) => response.headers.get('content-length')),
          });
        }
        await fetchEvent.updateUI({ title: 'Telechargement termine' });
        await notifyDriveClients({
          type: 'backgroundfetchsuccess',
          id: fetchEvent.registration.id,
          results,
        });
      })(),
    );
  });

  driveWorkerSelf.addEventListener('backgroundfetchfail', (event: BackgroundFetchEvent) => {
    const fetchEvent = event as BackgroundFetchEvent;
    fetchEvent.waitUntil(
      (async () => {
        await fetchEvent.updateUI({ title: 'Telechargement echoue' });
        await notifyDriveClients({
          type: 'backgroundfetchfail',
          id: fetchEvent.registration.id,
        });
      })(),
    );
  });

  driveWorkerSelf.addEventListener('backgroundfetchclick', (event: BackgroundFetchEvent) => {
    const fetchEvent = event as BackgroundFetchEvent;
    fetchEvent.waitUntil(driveWorkerSelf.clients.openWindow('/downloads'));
  });
}
