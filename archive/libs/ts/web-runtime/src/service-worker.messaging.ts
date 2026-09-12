const SERVICE_WORKER_TIMEOUT_MS = 5000;

/** Browser readiness is bounded independently of this module's registration helper. */
export async function getSwReady(): Promise<ServiceWorkerRegistration | null> {
  if (!('serviceWorker' in navigator)) return null;
  let timer: ReturnType<typeof setTimeout> | undefined;
  try {
    return await Promise.race([
      navigator.serviceWorker.ready,
      new Promise<never>((_, reject) => {
        timer = setTimeout(
          () => reject(new Error('Service worker readiness timed out')),
          SERVICE_WORKER_TIMEOUT_MS,
        );
      }),
    ]);
  } finally {
    clearTimeout(timer);
  }
}

/** At most 5s for readiness, then 5s for a reply. Timed-out requests are never replayed. */
export async function sendToSw<T = unknown>(
  type: string,
  data?: Record<string, unknown>,
): Promise<T> {
  const registration = await getSwReady();
  const active = registration?.active;
  if (!active) throw new Error('Service worker not active');
  const id = crypto.randomUUID();
  const channel = new MessageChannel();
  let timer: ReturnType<typeof setTimeout> | undefined;
  try {
    return await new Promise<T>((resolve, reject) => {
      timer = setTimeout(
        () => reject(new Error('Service worker response timed out')),
        SERVICE_WORKER_TIMEOUT_MS,
      );
      channel.port1.onmessage = (
        event: MessageEvent<{ id: string; result?: T; error?: string }>,
      ) => {
        if (event.data?.id !== id) return;
        if (event.data.error) reject(new Error(event.data.error));
        else resolve(event.data.result as T);
      };
      channel.port1.onmessageerror = () =>
        reject(new Error('Service worker response could not be decoded'));
      active.postMessage({ ...data, type, id }, [channel.port2]);
    });
  } finally {
    clearTimeout(timer);
    channel.port1.onmessage = null;
    channel.port1.onmessageerror = null;
    channel.port1.close();
    channel.port2.close();
  }
}
