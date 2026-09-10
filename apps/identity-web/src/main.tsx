import { createRoot } from 'react-dom/client';
import { flushSync } from 'react-dom';
import { App } from './app';
import { AuthorizationController } from './authorization.controller';
import { identityGateway } from './authorization.gateway';
import { RecoveryController, recoveryGateway } from './recovery.controller';
import './styles.css';

const root = document.getElementById('root');
if (!root) throw new Error('Missing application root');
const controller = new AuthorizationController(identityGateway(location.origin), (url) =>
  location.assign(url),
);
const recovery = new RecoveryController(recoveryGateway(location.origin));
window.addEventListener(
  'pagehide',
  () =>
    flushSync(() => {
      controller.dispose();
      recovery.dispose();
    }),
  { once: true },
);
// Outside the React lifecycle: a PAR request is consumed only once per document.
if (location.pathname === '/oauth/authorize') void controller.start(location.href);
if (location.pathname === '/recovery') void recovery.start();
createRoot(root).render(<App controller={controller} recovery={recovery} />);
