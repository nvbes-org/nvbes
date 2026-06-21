import {
  configureErrorReporting,
  registerServiceWorker,
  type ClientErrorReporter,
} from '@nvbes/web-runtime';
import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import { captureAnalyticsException, initAnalytics } from './identity.analytics';
import {
  captureErrorReportingException,
  initErrorReporting,
  syncErrorReportingConsent,
} from './identity.error.reporting';
import { TRACKING_CONSENT_CHANGED_EVENT } from './tracking-consent';
import './styles.css';

const errorReportingInitialized = initErrorReporting();
initAnalytics();
window.addEventListener(TRACKING_CONSENT_CHANGED_EVENT, () => {
  void syncErrorReportingConsent();
});

const clientErrorReporter: ClientErrorReporter = {
  captureException: (error, context) => {
    if (errorReportingInitialized) {
      captureErrorReportingException(error, context);
    }

    void captureAnalyticsException(error, {
      event_source: context.tags.source,
      error_kind: context.tags.feature,
    });
  },
};

configureErrorReporting({
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

registerServiceWorker();
