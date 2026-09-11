import { createRoot } from 'react-dom/client';
import { flushSync } from 'react-dom';
import { App } from './app';
import { AuthorizationController } from './authorization.controller';
import { identityGateway } from './authorization.gateway';
import { RecoveryController, recoveryGateway } from './recovery.controller';
import './styles.css';
import { LogoutController, logoutGateway } from './logout.controller';
import {
  PasswordRecoveryController,
  passwordRecoveryGateway,
  recoveryLink,
} from './password-recovery.controller';

const root = document.getElementById('root');
if (!root) throw new Error('Missing application root');
const controller = new AuthorizationController(identityGateway(location.origin), (url) =>
  location.assign(url),
);
const recovery = new RecoveryController(recoveryGateway(location.origin));
const passwordRecovery = new PasswordRecoveryController(
  passwordRecoveryGateway(location.origin),
  location.pathname === '/password-recovery' ? recoveryLink(new URL(location.href)) : undefined,
);
if (location.pathname === '/password-recovery')
  history.replaceState(null, '', '/password-recovery');
// A mail link opened in this same document may only change the fragment.
// Restart the document so bootstrap consumes it once, before any API request.
window.addEventListener('hashchange', () => {
  if (location.pathname === '/password-recovery' && location.hash) location.reload();
});
const logoutParams = new URLSearchParams(location.hash.slice(1));
const logoutRequest =
  location.pathname === '/logout' && location.hash
    ? logoutParams.size === 1 && logoutParams.has('request')
      ? logoutParams.get('request')!
      : ''
    : undefined;
if (location.pathname === '/logout') history.replaceState(null, '', '/logout');
const logout = new LogoutController(logoutGateway(location.origin, logoutRequest), (url) =>
  location.assign(url),
);
window.addEventListener(
  'pagehide',
  () =>
    flushSync(() => {
      controller.dispose();
      recovery.dispose();
      logout.dispose();
      passwordRecovery.dispose();
    }),
  { once: true },
);
// Outside the React lifecycle: a PAR request is consumed only once per document.
if (location.pathname === '/oauth/authorize') void controller.start(location.href);
if (location.pathname === '/recovery') void recovery.start();
if (location.pathname === '/logout') void logout.start();
if (location.pathname === '/password-recovery') void passwordRecovery.start();
createRoot(root).render(
  <App
    controller={controller}
    recovery={recovery}
    logout={logout}
    passwordRecovery={passwordRecovery}
  />,
);
