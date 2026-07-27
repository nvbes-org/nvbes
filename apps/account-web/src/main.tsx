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
import {
  captureFaroException,
  captureFaroNavigation,
  initFaro,
  syncFaroConsent,
} from './identity.faro';
import { router } from './identity.router';
import { TRACKING_CONSENT_CHANGED_EVENT } from './tracking-consent';
import './styles.css';

initErrorReporting();
initFaro();
initAnalytics();
router.subscribe('onResolved', ({ pathChanged, toLocation }) => {
  if (pathChanged) {
    captureFaroNavigation(toLocation.pathname);
  }
});
window.addEventListener(TRACKING_CONSENT_CHANGED_EVENT, () => {
  void syncErrorReportingConsent();
  syncFaroConsent();
});

const clientErrorReporter: ClientErrorReporter = {
  captureException: (error, context) => {
    void captureErrorReportingException(error, context);
    captureFaroException(error, context);

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
