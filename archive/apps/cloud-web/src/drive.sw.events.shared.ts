export type ExtendableMessageEventLike = {
  data?: unknown;
  ports?: MessagePort[];
  waitUntil(promise: Promise<unknown>): void;
};

export interface PeriodicSyncEvent {
  readonly tag: string;
  waitUntil(promise: Promise<unknown>): void;
}

interface BackgroundFetchRecord {
  readonly request: Request;
  readonly responseReady: Promise<Response>;
}

interface BackgroundFetchRegistration {
  readonly id: string;
  matchAll(): Promise<BackgroundFetchRecord[]>;
}

export interface BackgroundFetchEvent {
  readonly registration: BackgroundFetchRegistration;
  updateUI(options: { title: string }): Promise<void>;
  waitUntil(promise: Promise<unknown>): void;
}

type DriveClient = { postMessage(message: unknown): void };
type DriveClients = {
  matchAll(): Promise<DriveClient[]>;
  openWindow(url: string): Promise<unknown>;
};

export type DriveWorkerGlobal = typeof globalThis & {
  addEventListener(type: 'error', listener: (event: ErrorEvent) => void): void;
  addEventListener(
    type: 'unhandledrejection',
    listener: (event: PromiseRejectionEvent) => void,
  ): void;
  addEventListener(type: 'periodicsync', listener: (event: PeriodicSyncEvent) => void): void;
  addEventListener(type: 'message', listener: (event: ExtendableMessageEventLike) => void): void;
  addEventListener(
    type: 'backgroundfetchsuccess' | 'backgroundfetchfail' | 'backgroundfetchclick',
    listener: (event: BackgroundFetchEvent) => void,
  ): void;
  addEventListener(type: string, listener: (event: Event) => void): void;
  clients: DriveClients;
  skipWaiting(): Promise<void>;
};

export const driveWorkerSelf = globalThis as DriveWorkerGlobal;

export async function notifyDriveClients(message: Record<string, unknown>) {
  const allClients = await driveWorkerSelf.clients.matchAll();
  for (const client of allClients) {
    client.postMessage(message);
  }
}
