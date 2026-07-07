import { verifiedFetch } from '@nvbes/web-runtime';

export type ConnectionStatus = 'connected' | 'disconnected' | 'checking';

type Listener = (status: ConnectionStatus) => void;
type ApiMonitorGlobal = typeof globalThis & {
  __nvbesIdentityApiMonitor?: ApiMonitor;
};

const HEALTH_CHECK_TIMEOUT_MS = 5_000;

function createTimeoutSignal(timeoutMs: number): { signal: AbortSignal; clear: () => void } {
  const controller = new AbortController();
  const timeoutId = globalThis.setTimeout(() => {
    controller.abort();
  }, timeoutMs);

  return {
    signal: controller.signal,
    clear: () => globalThis.clearTimeout(timeoutId),
  };
}

class ApiMonitor {
  private status: ConnectionStatus = 'connected';
  private listeners: Set<Listener> = new Set();
  private probeIntervalId: number | null = null;
  private isChecking = false;
  private fetchImpl: typeof fetch | null = null;
  private originalFetch: typeof fetch | null = null;

  constructor() {
    this.setupFetchInterceptor();
  }

  subscribe(listener: Listener) {
    this.listeners.add(listener);
    listener(this.status);
    return () => {
      this.listeners.delete(listener);
    };
  }

  getStatus() {
    return this.status;
  }

  private setStatus(newStatus: ConnectionStatus) {
    if (this.status === newStatus) return;
    this.status = newStatus;
    this.listeners.forEach((listener) => listener(newStatus));

    if (newStatus === 'disconnected') {
      this.startProbing();
    } else if (newStatus === 'connected') {
      this.stopProbing();
    }
  }

  private setupFetchInterceptor() {
    if (typeof window === 'undefined') return;

    const originalFetch = window.fetch;
    this.originalFetch = originalFetch;
    this.fetchImpl = originalFetch.bind(window);
    window.fetch = async (input, init) => {
      const urlString =
        typeof input === 'string' ? input : input instanceof URL ? input.toString() : input.url;
      const isHealthCheck = urlString.endsWith('/health');

      try {
        const response = await originalFetch(input, init);
        if (!isHealthCheck) {
          if (response.status === 502 || response.status === 503 || response.status === 504) {
            this.setStatus('disconnected');
          } else {
            this.setStatus('connected');
          }
        }
        return response;
      } catch (error) {
        if (!isHealthCheck) {
          this.setStatus('disconnected');
        }
        throw error;
      }
    };
  }

  private startProbing() {
    if (this.probeIntervalId !== null) return;

    this.probeIntervalId = window.setInterval(async () => {
      if (this.isChecking) return;
      this.isChecking = true;
      try {
        const identityApiBaseUrl =
          import.meta.env.VITE_IDENTITY_API_BASE_URL || 'http://localhost:4000';
        const healthUrl = `${identityApiBaseUrl}/health`;
        const timeout = createTimeoutSignal(HEALTH_CHECK_TIMEOUT_MS);
        const response = await this.fetchHealth(healthUrl, identityApiBaseUrl, timeout);
        if (response.ok) {
          this.setStatus('connected');
        }
      } catch {
        // Still unreachable
      } finally {
        this.isChecking = false;
      }
    }, 5000);
  }

  private stopProbing() {
    if (this.probeIntervalId !== null) {
      clearInterval(this.probeIntervalId);
      this.probeIntervalId = null;
    }
  }

  dispose() {
    this.stopProbing();
    this.listeners.clear();
    this.isChecking = false;

    if (typeof window !== 'undefined' && this.originalFetch) {
      window.fetch = this.originalFetch;
    }

    this.fetchImpl = null;
    this.originalFetch = null;
  }

  async checkConnectionNow() {
    if (this.isChecking) return false;
    this.isChecking = true;
    this.setStatus('checking');
    try {
      const identityApiBaseUrl =
        import.meta.env.VITE_IDENTITY_API_BASE_URL || 'http://localhost:4000';
      const healthUrl = `${identityApiBaseUrl}/health`;
      const timeout = createTimeoutSignal(HEALTH_CHECK_TIMEOUT_MS);
      const response = await this.fetchHealth(healthUrl, identityApiBaseUrl, timeout);
      if (response.ok) {
        this.setStatus('connected');
        return true;
      }
    } catch {
      // Unreachable
    } finally {
      this.isChecking = false;
      if (this.status === 'checking') {
        this.setStatus('disconnected');
      }
    }
    return false;
  }

  private async fetchHealth(
    healthUrl: string,
    identityApiBaseUrl: string,
    timeout: { signal: AbortSignal; clear: () => void },
  ): Promise<Response> {
    try {
      return await verifiedFetch(healthUrl, {
        allowedOrigins: [identityApiBaseUrl],
        fetchImpl: this.fetchImpl ?? undefined,
        method: 'GET',
        cache: 'no-store',
        signal: timeout.signal,
      });
    } finally {
      timeout.clear();
    }
  }
}

const apiMonitorGlobal = globalThis as ApiMonitorGlobal;
apiMonitorGlobal.__nvbesIdentityApiMonitor?.dispose();

export const apiMonitor = new ApiMonitor();
apiMonitorGlobal.__nvbesIdentityApiMonitor = apiMonitor;

if (import.meta.hot) {
  import.meta.hot.dispose(() => {
    apiMonitor.dispose();
    if (apiMonitorGlobal.__nvbesIdentityApiMonitor === apiMonitor) {
      delete apiMonitorGlobal.__nvbesIdentityApiMonitor;
    }
  });
}
