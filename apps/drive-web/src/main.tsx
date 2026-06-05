import React from 'react';
import ReactDOM from 'react-dom/client';
import { App } from './App';
import './styles.css';
import { configureErrorReporting, registerServiceWorker } from '@nvbes/web-runtime';
import { initPostHog } from './drive.posthog';
import { captureSentryException, initSentry } from './drive.sentry';
import { syncTrackingConsent } from './tracking-consent';

const sentryInitialized = initSentry();
initPostHog();

void syncTrackingConsent();

configureErrorReporting({
  cloudflare: import.meta.env.VITE_CLOUDFLARE_REPORTING_ENABLED === 'true',
  reporter: sentryInitialized ? { captureException: captureSentryException } : null,
});

const rootEl = document.getElementById('root');
if (!rootEl) {
  throw new Error('Root element #root not found');
}

ReactDOM.createRoot(rootEl).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);

registerServiceWorker();
