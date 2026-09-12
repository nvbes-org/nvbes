type WorkerRequest = {
  id: number;
  method: string;
  args: unknown[];
};

type WorkerResponse = {
  id: number;
  result?: unknown;
  error?: string;
};

export function defineWorker<M extends Record<string, (...args: never[]) => unknown>>(handlers: M) {
  self.onmessage = async (event: MessageEvent<WorkerRequest>) => {
    const { id, method, args } = event.data;
    const handler = handlers[method] as ((...args: unknown[]) => unknown) | undefined;
    if (!handler) {
      self.postMessage({ id, error: `Unknown method: ${method}` });
      return;
    }
    try {
      const result = await handler(...args);
      self.postMessage({ id, result });
    } catch (err) {
      self.postMessage({ id, error: err instanceof Error ? err.message : String(err) });
    }
  };
}

export function createWorker<M extends Record<string, (...args: never[]) => unknown>>(
  workerFactory: () => Worker,
): { [K in keyof M]: (...args: Parameters<M[K]>) => Promise<Awaited<ReturnType<M[K]>>> } {
  let nextId = 1;
  const pending = new Map<
    number,
    { resolve: (value: unknown) => void; reject: (error: Error) => void }
  >();
  let worker: Worker | null = null;

  function getWorker(): Worker {
    if (!worker) {
      worker = workerFactory();
      worker.onmessage = (event: MessageEvent<WorkerResponse>) => {
        const { id, result, error } = event.data;
        const p = pending.get(id);
        if (!p) return;
        pending.delete(id);
        if (error) {
          p.reject(new Error(error));
        } else {
          p.resolve(result);
        }
      };
      worker.onerror = (event) => {
        for (const [, p] of pending) {
          p.reject(new Error(event.message || 'Worker error'));
        }
        pending.clear();
      };
    }
    return worker;
  }

  return new Proxy({} as Record<PropertyKey, never>, {
    get(_target, method: string) {
      return (...args: unknown[]) => {
        return new Promise((resolve, reject) => {
          const id = nextId++;
          pending.set(id, { resolve, reject });
          getWorker().postMessage({ id, method, args });
        });
      };
    },
  }) as { [K in keyof M]: (...args: Parameters<M[K]>) => Promise<Awaited<ReturnType<M[K]>>> };
}
