export type ConnectionStatus = 'connected' | 'disconnected' | 'checking';

type Listener = (status: ConnectionStatus) => void;

class ApiMonitor {
  private status: ConnectionStatus = 'connected';
  private listeners: Set<Listener> = new Set();
  private probeIntervalId: number | null = null;
  private isChecking = false;

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
          import.meta.env.VITE_IDENTITY_API_BASE_URL || window.location.origin;
        const response = await fetch(`${identityApiBaseUrl}/health`, {
          method: 'GET',
          cache: 'no-store',
        });
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

  async checkConnectionNow() {
    if (this.isChecking) return false;
    this.isChecking = true;
    this.setStatus('checking');
    try {
      const identityApiBaseUrl =
        import.meta.env.VITE_IDENTITY_API_BASE_URL || window.location.origin;
      const response = await fetch(`${identityApiBaseUrl}/health`, {
        method: 'GET',
        cache: 'no-store',
      });
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
}

export const apiMonitor = new ApiMonitor();
