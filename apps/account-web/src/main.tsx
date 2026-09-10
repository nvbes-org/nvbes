import { createRoot } from 'react-dom/client';
import { flushSync } from 'react-dom';
import { App } from './app';
import { AccountController } from './account.controller';
import { accountGateway } from './account.gateway';
import './styles.css';
import { BrowserLogoutTransaction, submitLogout } from '@nvbes/identity-sdk-web/oauth';

const root = document.getElementById('root');
if (!root) throw new Error('Missing application root');
const controller = new AccountController(
  accountGateway(location.origin),
  (url) => location.assign(url),
  submitLogout,
);
const callback = location.pathname === '/oauth/callback' ? new URL(location.href) : undefined;
// Remove authorization codes before rendering or making any network request.
if (callback) history.replaceState(null, '', '/');
let logoutResult: 'complete' | 'invalid' | undefined;
if (location.pathname === '/oauth/logout/callback') {
  const url = new URL(location.href);
  history.replaceState(null, '', '/');
  try {
    new BrowserLogoutTransaction(sessionStorage).consume(url);
    logoutResult = 'complete';
  } catch {
    logoutResult = 'invalid';
  }
}
window.addEventListener('pagehide', () => flushSync(() => controller.dispose()), { once: true });
void controller.start(callback, logoutResult);
window.addEventListener('focus', () => {
  if (document.visibilityState === 'visible') void controller.revalidate();
});
document.addEventListener('visibilitychange', () => {
  if (document.visibilityState === 'visible') void controller.revalidate();
});
createRoot(root).render(<App controller={controller} />);
