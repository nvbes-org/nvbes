import { configureErrorReporting, type ClientErrorReporter } from '@nvbes/web-runtime';
import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import { capturePostHogException, initPostHog } from './identity.posthog';
import { captureSentryException, initSentry } from './identity.sentry';
import { syncTrackingConsent } from './tracking-consent';
import './styles.css';

const sentryInitialized = initSentry();
initPostHog();

void syncTrackingConsent();

const clientErrorReporter: ClientErrorReporter = {
  captureException: (error, context) => {
    if (sentryInitialized) {
      captureSentryException(error, context);
    }

    void capturePostHogException(error, {
      event_source: context.tags.source,
      error_kind: context.tags.feature,
    });
  },
};

configureErrorReporting({
  cloudflare: import.meta.env.VITE_CLOUDFLARE_REPORTING_ENABLED === 'true',
  reporter: clientErrorReporter,
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
