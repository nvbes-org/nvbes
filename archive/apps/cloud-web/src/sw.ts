/// <reference lib="webworker" />

import { clientsClaim } from 'workbox-core';
import { registerDriveServiceWorkerEvents } from './drive.sw.events';
import { registerDriveServiceWorkerRoutes } from './drive.sw.routing';

let isSlowConnection = false;

clientsClaim();
registerDriveServiceWorkerRoutes(isSlowConnection);
registerDriveServiceWorkerEvents({
  getIsSlowConnection: () => isSlowConnection,
  setIsSlowConnection: (value) => {
    isSlowConnection = value;
  },
});
