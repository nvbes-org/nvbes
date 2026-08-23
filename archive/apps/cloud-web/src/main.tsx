import React from 'react';
import ReactDOM from 'react-dom/client';
import { App } from './App';
import './styles.css';
import {
  configureErrorReporting,
  registerServiceWorker,
  type ClientErrorReporter,
} from '@nvbes/web-runtime';
import { captureAnalyticsException, initAnalytics } from './drive.analytics';
import {
  captureErrorReportingException,
  initErrorReporting,
  syncErrorReportingConsent,
} from './drive.error.reporting';
import { syncDriveServiceWorkerErrorReportingConsent } from './drive.sw.consent';
import { syncTrackingConsent, TRACKING_CONSENT_CHANGED_EVENT } from './tracking-consent';

const errorReportingInitialized = initErrorReporting();
initAnalytics();

void syncTrackingConsent();
window.addEventListener(TRACKING_CONSENT_CHANGED_EVENT, () => {
  void syncErrorReportingConsent();
  void syncDriveServiceWorkerErrorReportingConsent();
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
void syncDriveServiceWorkerErrorReportingConsent();
